//! Привязки FFI к whisper.dll (нативная библиотека whisper.cpp).
//! Используется libloading для динамической загрузки — линковка на этапе компиляции не нужна.

use std::ffi::{c_char, c_float, c_int, CStr, CString};
use std::path::Path;

use libloading::{Library, Symbol};

// Windows API: SetDllDirectoryW — добавляет папку в пути поиска DLL.
// whisper.dll зависит от ggml.dll, SDL2.dll, parakeet.dll в той же папке.
// SetDllDirectoryW нужно, чтобы Windows нашла их до LoadLibraryExW.
#[cfg(windows)]
extern "system" {
    fn SetDllDirectoryW(lpPathName: *const u16) -> i32;
}

#[cfg(windows)]
fn add_dll_directory(path: &Path) {
    use std::os::windows::ffi::OsStrExt;
    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        SetDllDirectoryW(wide.as_ptr());
    }
}

#[cfg(not(windows))]
fn add_dll_directory(_path: &Path) {}

/// Непрозрачный контекст whisper (держит загруженную модель в памяти).
#[repr(C)]
pub struct WhisperContext {
    _private: [u8; 0],
}

/// Параметры инициализации контекста (whisper_context_params из whisper.h).
///
/// Раскладка (20 байт, проверено через Python ctypes):
///   bool use_gpu;       // смещение 0  (1 байт)
///   bool flash_attn;    // смещение 1  (1 байт)
///   // 2 байта выравнивания
///   int  gpu_device;    // смещение 4  (4 байта)
///   bool dtw;           // смещение 8  (1 байт)
///   // 3 байта выравнивания
///   int  devices;       // смещение 12 (4 байта) — битовая маска, НЕ указатель
///   int  backends;      // смещение 16 (4 байта) — битовая маска, НЕ указатель
/// Итого: 20 байт.
#[repr(C)]
pub struct WhisperContextParams {
    pub use_gpu: u8,       // смещение 0
    pub flash_attn: u8,    // смещение 1
    pub _pad1: [u8; 2],    // смещение 2-3
    pub gpu_device: c_int, // смещение 4-7
    pub dtw: u8,           // смещение 8
    pub _pad2: [u8; 3],    // смещение 9-11
    pub devices: c_int,    // смещение 12-15
    pub backends: c_int,   // смещение 16-19
}

/// Стратегии сэмплирования для whisper.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub enum WhisperSamplingStrategy {
    Greedy = 0,
    BeamSearch = 1,
}

/// Полные параметры для whisper_full().
#[repr(C)]
pub struct WhisperFullParams {
    pub strategy: c_int,
    pub n_threads: c_int,
    pub n_max_text_ctx: c_int,
    pub offset_ms: c_int,
    pub duration_ms: c_int,
    pub translate: c_int,
    pub no_context: c_int,
    pub no_timestamps: c_int,
    pub single_segment: c_int,
    pub print_special: c_int,
    pub print_progress: c_int,
    pub print_realtime: c_int,
    pub print_timestamps: c_int,
    pub token_timestamps: c_int,
    pub thold_pt: c_float,
    pub thold_ptsum: c_float,
    pub max_len: c_int,
    pub split_on_word: c_int,
    pub max_tokens: c_int,
    pub speed_up: c_int,
    pub debug_mode: c_int,
    pub audio_ctx: c_int,
    pub tdrz_enable: c_int,
    pub initial_prompt: *const c_char,
    pub prompt_tokens: *const c_int,
    pub prompt_n_tokens: c_int,
    pub language: *const c_char,
    pub detect_language: c_int,
    pub suppress_blank: c_int,
    pub suppress_non_speech_tokens: c_int,
    pub temperature: c_float,
    pub max_initial_ts: c_float,
    pub length_penalty: c_float,
    pub temperature_inc: c_float,
    pub entropy_thold: c_float,
    pub logprob_thold: c_float,
    pub no_speech_thold: c_float,
    pub greedy: c_int,
    pub beam_search: c_int,
    pub n_best: c_int,
}

/// Загруженная whisper.dll с указателями на функции.
pub struct WhisperDll {
    _lib: Library,
    pub context_default_params_by_ref:
        unsafe extern "C" fn(*mut WhisperContextParams),
    pub init_from_file_with_params_no_state:
        unsafe extern "C" fn(*const c_char, *const WhisperContextParams) -> *mut WhisperContext,
    pub free: unsafe extern "C" fn(*mut WhisperContext),
    pub full_default_params: unsafe extern "C" fn(c_int, *mut WhisperFullParams),
    pub full: unsafe extern "C" fn(*mut WhisperContext, WhisperFullParams, *const c_float, c_int) -> c_int,
    pub full_n_segments: unsafe extern "C" fn(*mut WhisperContext) -> c_int,
    pub full_get_segment_text: unsafe extern "C" fn(*mut WhisperContext, c_int) -> *const c_char,
}

impl WhisperDll {
    /// Загружает whisper.dll из указанной папки.
    pub fn load(dll_dir: &Path) -> Result<Self, String> {
        let dll_path = dll_dir.join("whisper.dll");
        if !dll_path.exists() {
            return Err(format!("whisper.dll not found at {}", dll_path.display()));
        }

        // Добавляем папку DLL в пути поиска, чтобы whisper.dll нашла свои
        // зависимости (ggml.dll, SDL2.dll, parakeet.dll) в той же папке.
        // Путь должен быть абсолютным — SetDllDirectoryW этого требует.
        let dll_dir_abs = dll_dir
            .canonicalize()
            .unwrap_or_else(|_| dll_dir.to_path_buf());
        add_dll_directory(&dll_dir_abs);

        // Приводим путь DLL к абсолютному — libloading использует
        // LOAD_WITH_ALTERED_SEARCH_PATH для путей с разделителями,
        // что говорит Windows искать зависимости в папке DLL.
        let dll_path_abs = dll_path
            .canonicalize()
            .unwrap_or_else(|_| dll_path.clone());

        // БЕЗОПАСНОСТЬ: libloading грузит нативную библиотеку. Доверяем whisper.dll.
        let lib = unsafe {
            Library::new(&dll_path_abs).map_err(|e| format!("Failed to load whisper.dll: {e}"))?
        };

        // БЕЗОПАСНОСТЬ: Грузим символы по точным C-именам из whisper.h.
        // Сигнатуры функций должны точно совпадать с C ABI.
        // Извлекаем сырые указатели и дропаем Symbols до перемещения `lib`.
        let context_default_params_by_ref: unsafe extern "C" fn(*mut WhisperContextParams) = {
            let sym: Symbol<unsafe extern "C" fn(*mut WhisperContextParams)> = unsafe {
                lib.get(b"whisper_context_default_params_by_ref\0")
                    .map_err(|e| format!("Symbol whisper_context_default_params_by_ref: {e}"))?
            };
            *sym
        };

        let init_from_file_with_params_no_state: unsafe extern "C" fn(
            *const c_char,
            *const WhisperContextParams,
        ) -> *mut WhisperContext = {
            let sym: Symbol<
                unsafe extern "C" fn(*const c_char, *const WhisperContextParams) -> *mut WhisperContext,
            > = unsafe {
                lib.get(b"whisper_init_from_file_with_params_no_state\0")
                    .map_err(|e| format!("Symbol whisper_init_from_file_with_params_no_state: {e}"))?
            };
            *sym
        };

        let free: unsafe extern "C" fn(*mut WhisperContext) = {
            let sym: Symbol<unsafe extern "C" fn(*mut WhisperContext)> = unsafe {
                lib.get(b"whisper_free\0")
                    .map_err(|e| format!("Symbol whisper_free: {e}"))?
            };
            *sym
        };

        let full_default_params: unsafe extern "C" fn(c_int, *mut WhisperFullParams) = {
            let sym: Symbol<unsafe extern "C" fn(c_int, *mut WhisperFullParams)> = unsafe {
                lib.get(b"whisper_full_default_params_by_ref\0")
                    .map_err(|e| format!("Symbol whisper_full_default_params_by_ref: {e}"))?
            };
            *sym
        };

        let full: unsafe extern "C" fn(
            *mut WhisperContext,
            WhisperFullParams,
            *const c_float,
            c_int,
        ) -> c_int = {
            let sym: Symbol<
                unsafe extern "C" fn(
                    *mut WhisperContext,
                    WhisperFullParams,
                    *const c_float,
                    c_int,
                ) -> c_int,
            > = unsafe {
                lib.get(b"whisper_full\0")
                    .map_err(|e| format!("Symbol whisper_full: {e}"))?
            };
            *sym
        };

        let full_n_segments: unsafe extern "C" fn(*mut WhisperContext) -> c_int = {
            let sym: Symbol<unsafe extern "C" fn(*mut WhisperContext) -> c_int> = unsafe {
                lib.get(b"whisper_full_n_segments\0")
                    .map_err(|e| format!("Symbol whisper_full_n_segments: {e}"))?
            };
            *sym
        };

        let full_get_segment_text: unsafe extern "C" fn(
            *mut WhisperContext,
            c_int,
        ) -> *const c_char = {
            let sym: Symbol<
                unsafe extern "C" fn(*mut WhisperContext, c_int) -> *const c_char,
            > = unsafe {
                lib.get(b"whisper_full_get_segment_text\0")
                    .map_err(|e| format!("Symbol whisper_full_get_segment_text: {e}"))?
            };
            *sym
        };

        Ok(Self {
            _lib: lib,
            context_default_params_by_ref,
            init_from_file_with_params_no_state,
            free,
            full_default_params,
            full,
            full_n_segments,
            full_get_segment_text,
        })
    }

    /// Создаёт контекст whisper из файла модели с параметрами только-CPU.
    /// Модель остаётся в памяти до вызова `free_context`.
    pub unsafe fn init_context(&self, model_path: &str) -> Result<*mut WhisperContext, String> {
        let c_path = CString::new(model_path)
            .map_err(|e| format!("Invalid model path: {e}"))?;

        // Собираем параметры контекста только-CPU.
        // use_gpu = 0 (false) — только CPU
        // backends = 1 (битовая маска GGML_BACKEND_TYPE_CPU)
        // devices = 1 (хотя бы одно устройство должно быть заявлено)
        let ctx_params = WhisperContextParams {
            use_gpu: 0,
            flash_attn: 0,
            _pad1: [0; 2],
            gpu_device: 0,
            dtw: 0,
            _pad2: [0; 3],
            devices: 1,
            backends: 1,
        };

        let ctx = (self.init_from_file_with_params_no_state)(c_path.as_ptr(), &ctx_params);
        if ctx.is_null() {
            return Err(format!(
                "whisper_init_from_file_with_params_no_state failed for {}",
                model_path
            ));
        }
        Ok(ctx)
    }

    /// Освобождает контекст whisper и выгружает модель из памяти.
    pub unsafe fn free_context(&self, ctx: *mut WhisperContext) {
        if !ctx.is_null() {
            (self.free)(ctx);
        }
    }

    /// Создаёт параметры по умолчанию для whisper_full().
    /// whisper_full_default_params_by_ref заполняет структуру по указателю (возвращает void).
    pub unsafe fn default_params(&self, strategy: WhisperSamplingStrategy) -> WhisperFullParams {
        let mut params: WhisperFullParams = std::mem::zeroed();
        (self.full_default_params)(strategy as c_int, &mut params);
        params
    }

    /// Запускает инференс на звуковых образцах.
    /// Возвращает 0 при успехе, не ноль при ошибке.
    pub unsafe fn run_full(
        &self,
        ctx: *mut WhisperContext,
        params: WhisperFullParams,
        samples: &[f32],
    ) -> Result<i32, String> {
        let ret = (self.full)(ctx, params, samples.as_ptr(), samples.len() as c_int);
        if ret != 0 {
            return Err(format!("whisper_full returned error code {ret}"));
        }
        Ok(ret)
    }

    /// Возвращает количество сегментов после успешного whisper_full().
    pub unsafe fn n_segments(&self, ctx: *mut WhisperContext) -> i32 {
        (self.full_n_segments)(ctx) as i32
    }

    /// Возвращает текст сегмента по индексу.
    pub unsafe fn segment_text(&self, ctx: *mut WhisperContext, index: i32) -> String {
        let ptr = (self.full_get_segment_text)(ctx, index as c_int);
        if ptr.is_null() {
            return String::new();
        }
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}

/// Преобразует i16 образцы звука в f32 (нужно для whisper).
pub fn i16_to_f32(samples: &[i16]) -> Vec<f32> {
    samples
        .iter()
        .map(|&s| s as f32 / 32768.0)
        .collect()
}
