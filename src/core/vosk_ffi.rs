//! Привязки FFI к библиотеке распознавания речи VOSK (libvosk.dll / vosk.dll).
//! Используется libloading для динамической загрузки — линковка на этапе компиляции не нужна.
//!
//! Документация VOSK API: https://alphacephei.com/vosk/adaptation
//!
//! Если vosk.dll не найдена локально, она скачивается автоматически с GitHub Releases
//! и кладётся в папку с моделью.

use std::ffi::{c_char, c_float, c_int, c_short, CStr, CString};
use std::io;
use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

/// Загружает DLL с полным разрешением зависимостей.
/// На Windows используется LoadLibraryExW с LOAD_WITH_ALTERED_SEARCH_PATH,
/// чтобы зависимые DLL (libstdc++-6.dll и т.д.) искались в папке с DLL.
/// На других платформах — обычный libloading::Library::new.
fn load_library_with_deps(path: &Path) -> Result<Library, String> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        // LOAD_WITH_ALTERED_SEARCH_PATH = 0x8:
        //   Если путь абсолютный — сначала ищем зависимости в папке DLL,
        //   потом уже по стандартным системным путям.
        const LOAD_WITH_ALTERED_SEARCH_PATH: u32 = 0x8;
        let handle = unsafe {
            windows_sys::Win32::System::LibraryLoader::LoadLibraryExW(
                wide.as_ptr(),
                std::ptr::null_mut(),
                LOAD_WITH_ALTERED_SEARCH_PATH,
            )
        };
        if handle.is_null() {
            return Err(format!(
                "LoadLibraryExW failed for {}",
                path.display()
            ));
        }
        // БЕЗОПАСНОСТЬ: handle — валидный HMODULE из LoadLibraryExW.
        // libloading вызовет FreeLibrary при дропе.
        use libloading::os::windows::Library as WindowsLibrary;
        let win_lib = unsafe { WindowsLibrary::from_raw(handle as isize) };
        Ok(win_lib.into())
    }
    #[cfg(not(windows))]
    {
        // На Linux/macOS dlopen ищет зависимые .so в LD_LIBRARY_PATH,
        // RPATH и системных путях — но НЕ в папке загружаемой библиотеки.
        // Временно добавляем папку библиотеки в начало LD_LIBRARY_PATH,
        // чтобы нашлись bundled-зависимости (например, зависимости libvosk.so).
        let lib_dir = path
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or(".");
        let old_ld_path = std::env::var("LD_LIBRARY_PATH").unwrap_or_default();
        let new_ld_path = if old_ld_path.is_empty() {
            lib_dir.to_string()
        } else {
            format!("{}:{}", lib_dir, old_ld_path)
        };
        std::env::set_var("LD_LIBRARY_PATH", &new_ld_path);

        let result = unsafe {
            Library::new(path).map_err(|e| format!("Library::new failed: {e}"))
        };

        // Восстанавливаем исходный LD_LIBRARY_PATH.
        if old_ld_path.is_empty() {
            std::env::remove_var("LD_LIBRARY_PATH");
        } else {
            std::env::set_var("LD_LIBRARY_PATH", &old_ld_path);
        }

        result
    }
}


/// Непрозрачная VOSK-модель (загружена из папки с моделью).
#[repr(C)]
pub struct VoskModel {
    _private: [u8; 0],
}

/// Непрозрачный VOSK-распознаватель (создаётся из модели, обрабатывает звук).
#[repr(C)]
pub struct VoskRecognizer {
    _private: [u8; 0],
}

/// Загруженная VOSK DLL с указателями на функции.
pub struct VoskDll {
    _lib: Library,
    pub model_new: unsafe extern "C" fn(*const c_char) -> *mut VoskModel,
    pub model_free: unsafe extern "C" fn(*mut VoskModel),
    pub recognizer_new: unsafe extern "C" fn(*mut VoskModel, c_float) -> *mut VoskRecognizer,
    pub recognizer_accept_waveform:
        unsafe extern "C" fn(*mut VoskRecognizer, *const c_short, c_int) -> c_int,
    pub recognizer_result: unsafe extern "C" fn(*mut VoskRecognizer) -> *const c_char,
    pub recognizer_set_words: unsafe extern "C" fn(*mut VoskRecognizer, c_int),
    pub recognizer_set_max_alternatives: unsafe extern "C" fn(*mut VoskRecognizer, c_int),
    pub recognizer_free: unsafe extern "C" fn(*mut VoskRecognizer),
}

impl VoskDll {
    /// Загружает VOSK DLL. Если не найдена локально — качает с GitHub.
    /// Ищет в: папке модели, родительской папке, папке exe, cwd, whisper_model/.
    pub fn load(model_dir: &Path) -> Result<Self, String> {
        // Сначала пробуем найти DLL локально.
        if let Ok(dll) = Self::try_load_local(model_dir) {
            return Ok(dll);
        }

        // DLL не найдена — качаем в папку модели.
        let target = model_dir.join("vosk.dll");
        match ensure_vosk_dll(&target) {
            Ok(path) => {
                let lib = load_library_with_deps(&path)?;
                Self::from_library(lib)
            }
            Err(e) => Err(format!(
                "VOSK DLL not found and auto-download failed: {e}"
            )),
        }
    }

    /// Пытается найти и загрузить VOSK DLL из локальных путей.
    fn try_load_local(model_dir: &Path) -> Result<Self, String> {
        let dll_names = ["vosk.dll", "libvosk.dll"];

        let mut search_dirs: Vec<PathBuf> = vec![model_dir.to_path_buf()];
        if let Some(parent) = model_dir.parent() {
            search_dirs.push(parent.to_path_buf());
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(exe_dir) = exe.parent() {
                search_dirs.push(exe_dir.to_path_buf());
            }
        }
        if let Ok(cwd) = std::env::current_dir() {
            search_dirs.push(cwd);
        }
        search_dirs.push(PathBuf::from("whisper_model"));

        for dir in &search_dirs {
            for name in &dll_names {
                let candidate = dir.join(name);
                if candidate.exists() {
                    let dll_path_abs = candidate.canonicalize().unwrap_or(candidate);
                    let lib = load_library_with_deps(&dll_path_abs)?;
                    return Self::from_library(lib);
                }
            }
        }

        Err("not found locally".into())
    }

    /// Собирает VoskDll из уже загруженной Library.
    fn from_library(lib: Library) -> Result<Self, String> {
        // БЕЗОПАСНОСТЬ: Грузим символы по их точным C-именам из VOSK API.
        let model_new: unsafe extern "C" fn(*const c_char) -> *mut VoskModel = {
            let sym: Symbol<unsafe extern "C" fn(*const c_char) -> *mut VoskModel> = unsafe {
                lib.get(b"vosk_model_new\0")
                    .map_err(|e| format!("Symbol vosk_model_new: {e}"))?
            };
            *sym
        };

        let model_free: unsafe extern "C" fn(*mut VoskModel) = {
            let sym: Symbol<unsafe extern "C" fn(*mut VoskModel)> = unsafe {
                lib.get(b"vosk_model_free\0")
                    .map_err(|e| format!("Symbol vosk_model_free: {e}"))?
            };
            *sym
        };

        let recognizer_new: unsafe extern "C" fn(*mut VoskModel, c_float) -> *mut VoskRecognizer = {
            let sym: Symbol<
                unsafe extern "C" fn(*mut VoskModel, c_float) -> *mut VoskRecognizer,
            > = unsafe {
                lib.get(b"vosk_recognizer_new\0")
                    .map_err(|e| format!("Symbol vosk_recognizer_new: {e}"))?
            };
            *sym
        };

        let recognizer_accept_waveform: unsafe extern "C" fn(
            *mut VoskRecognizer,
            *const c_short,
            c_int,
        ) -> c_int = {
            let sym: Symbol<
                unsafe extern "C" fn(*mut VoskRecognizer, *const c_short, c_int) -> c_int,
            > = unsafe {
                lib.get(b"vosk_recognizer_accept_waveform\0")
                    .map_err(|e| format!("Symbol vosk_recognizer_accept_waveform: {e}"))?
            };
            *sym
        };

        let recognizer_result: unsafe extern "C" fn(
            *mut VoskRecognizer,
        ) -> *const c_char = {
            let sym: Symbol<
                unsafe extern "C" fn(*mut VoskRecognizer) -> *const c_char,
            > = unsafe {
                lib.get(b"vosk_recognizer_result\0")
                    .map_err(|e| format!("Symbol vosk_recognizer_result: {e}"))?
            };
            *sym
        };

        let recognizer_set_words: unsafe extern "C" fn(*mut VoskRecognizer, c_int) = {
            let sym: Symbol<unsafe extern "C" fn(*mut VoskRecognizer, c_int)> = unsafe {
                lib.get(b"vosk_recognizer_set_words\0")
                    .map_err(|e| format!("Symbol vosk_recognizer_set_words: {e}"))?
            };
            *sym
        };

        let recognizer_set_max_alternatives: unsafe extern "C" fn(*mut VoskRecognizer, c_int) = {
            let sym: Symbol<unsafe extern "C" fn(*mut VoskRecognizer, c_int)> = unsafe {
                lib.get(b"vosk_recognizer_set_max_alternatives\0")
                    .map_err(|e| format!("Symbol vosk_recognizer_set_max_alternatives: {e}"))?
            };
            *sym
        };

        let recognizer_free: unsafe extern "C" fn(*mut VoskRecognizer) = {
            let sym: Symbol<unsafe extern "C" fn(*mut VoskRecognizer)> = unsafe {
                lib.get(b"vosk_recognizer_free\0")
                    .map_err(|e| format!("Symbol vosk_recognizer_free: {e}"))?
            };
            *sym
        };

        Ok(Self {
            _lib: lib,
            model_new,
            model_free,
            recognizer_new,
            recognizer_accept_waveform,
            recognizer_result,
            recognizer_set_words,
            recognizer_set_max_alternatives,
            recognizer_free,
        })
    }

    /// Загружает VOSK-модель из указанной папки.
    /// В папке должны быть файлы модели (conf/, ivector/ и т.д.).
    pub unsafe fn load_model(&self, model_dir: &str) -> Result<*mut VoskModel, String> {
        let c_path = CString::new(model_dir)
            .map_err(|e| format!("Invalid model path: {e}"))?;

        let model = (self.model_new)(c_path.as_ptr());
        if model.is_null() {
            return Err(format!(
                "vosk_model_new failed for directory: {}",
                model_dir
            ));
        }
        Ok(model)
    }

    /// Освобождает VOSK-модель.
    pub unsafe fn free_model(&self, model: *mut VoskModel) {
        if !model.is_null() {
            (self.model_free)(model);
        }
    }

    /// Создаёт распознаватель из загруженной модели.
    /// sample_rate: обычно 16000.0 для 16 кГц звука.
    /// Также настраивает SetWords(true) и SetMaxAlternatives(0) для лучшей точности.
    pub unsafe fn create_recognizer(
        &self,
        model: *mut VoskModel,
        sample_rate: f32,
    ) -> Result<*mut VoskRecognizer, String> {
        let rec = (self.recognizer_new)(model, sample_rate as c_float);
        if rec.is_null() {
            return Err("vosk_recognizer_new returned NULL".into());
        }
        // Включаем информацию о таймингах слов — улучшает качество распознавания.
        (self.recognizer_set_words)(rec, 1);
        // Отключаем альтернативные гипотезы — только один лучший результат.
        (self.recognizer_set_max_alternatives)(rec, 0);
        Ok(rec)
    }

    /// Скармливает звуковые образцы распознавателю.
    /// `samples` — 16-битный знаковый PCM, моно, с частотой дискретизации,
    /// указанной при создании распознавателя.
    /// Возвращает 1 если данных достаточно для результата, иначе 0.
    pub unsafe fn accept_waveform(
        &self,
        rec: *mut VoskRecognizer,
        samples: &[i16],
    ) -> i32 {
        (self.recognizer_accept_waveform)(rec, samples.as_ptr(), samples.len() as c_int)
    }

    /// Забирает финальный результат распознавания как JSON-строку.
    /// Вызывать после того, как `accept_waveform` вернул 1.
    /// Строка принадлежит распознавателю — копируем сразу.
    pub unsafe fn result(&self, rec: *mut VoskRecognizer) -> String {
        let ptr = (self.recognizer_result)(rec);
        if ptr.is_null() {
            return String::new();
        }
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }

    /// Освобождает распознаватель.
    pub unsafe fn free_recognizer(&self, rec: *mut VoskRecognizer) {
        if !rec.is_null() {
            (self.recognizer_free)(rec);
        }
    }
}

/// Качает VOSK DLL с GitHub Releases, если её нет.
/// Возвращает путь к скачанной DLL.
fn ensure_vosk_dll(target: &Path) -> Result<PathBuf, String> {
    if target.exists() {
        return Ok(target.to_path_buf());
    }

    // Релизы VOSK: https://github.com/alphacep/vosk-api/releases
    // Используем проверенный тег. В zip лежит vosk.dll + зависимости.
    let release_tag = "v0.3.45";
    let asset = "vosk-win64-0.3.45.zip";
    let url = format!(
        "https://github.com/alphacep/vosk-api/releases/download/{release_tag}/{asset}"
    );

    // Качаем во временный файл.
    let tmp_zip = target.with_extension("zip.tmp");
    eprintln!("[VOSK] Downloading {url} ...");
    let response = reqwest::blocking::get(&url)
        .map_err(|e| format!("Download failed: {e}"))?;
    let bytes = response
        .bytes()
        .map_err(|e| format!("Read response failed: {e}"))?;
    std::fs::write(&tmp_zip, &bytes)
        .map_err(|e| format!("Write temp zip failed: {e}"))?;

    // Извлекаем vosk.dll из архива.
    eprintln!("[VOSK] Extracting vosk.dll ...");
    let file = std::fs::File::open(&tmp_zip)
        .map_err(|e| format!("Open zip failed: {e}"))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("Read zip failed: {e}"))?;

    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    let mut found_dll: Option<PathBuf> = None;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("Zip entry {i}: {e}"))?;
        let name = entry.name().to_lowercase();

        // Извлекаем vosk.dll / libvosk.dll и их зависимости.
        if name.ends_with(".dll") {
            let fname = Path::new(entry.name())
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| entry.name().to_string());
            let out_path = parent.join(&fname);
            if let Some(dir) = out_path.parent() {
                std::fs::create_dir_all(dir).ok();
            }
            let mut out = std::fs::File::create(&out_path)
                .map_err(|e| format!("Create {fname}: {e}"))?;
            io::copy(&mut entry, &mut out)
                .map_err(|e| format!("Extract {fname}: {e}"))?;
            eprintln!("[VOSK] Extracted: {fname}");
            if fname.eq_ignore_ascii_case("vosk.dll") || fname.eq_ignore_ascii_case("libvosk.dll") {
                found_dll = Some(out_path);
            }
        }
    }

    // Подчищаем временный zip.
    let _ = std::fs::remove_file(&tmp_zip);

    match found_dll {
        Some(path) => Ok(path),
        None => Err("vosk.dll / libvosk.dll not found in downloaded archive".into()),
    }
}
