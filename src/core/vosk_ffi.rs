//! FFI bindings to VOSK speech recognition library (libvosk.dll / vosk.dll).
//! Uses libloading for dynamic loading — no compile-time linking required.
//!
//! VOSK API reference: https://alphacephei.com/vosk/adaptation
//!
//! If vosk.dll is not found locally, it is auto-downloaded from GitHub releases
//! and placed into the model directory.

use std::ffi::{c_char, c_float, c_int, c_short, CStr, CString};
use std::io;
use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

/// Load a DLL with full dependency resolution.
/// On Windows, uses LoadLibraryExW with LOAD_WITH_ALTERED_SEARCH_PATH
/// so that dependent DLLs (libstdc++-6.dll, etc.) are found in the DLL's directory.
/// On other platforms, delegates to libloading::Library::new.
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
        //   If path is absolute, search the DLL's directory for dependencies first,
        //   then fall back to standard system paths.
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
        // SAFETY: handle is a valid HMODULE from LoadLibraryExW.
        // libloading will call FreeLibrary on drop.
        use libloading::os::windows::Library as WindowsLibrary;
        let win_lib = unsafe { WindowsLibrary::from_raw(handle as isize) };
        Ok(win_lib.into())
    }
    #[cfg(not(windows))]
    {
        // On Linux/macOS, dlopen searches for dependent .so in LD_LIBRARY_PATH,
        // RPATH, and system paths — but NOT in the loaded library's directory.
        // Temporarily prepend the library's directory to LD_LIBRARY_PATH
        // so that bundled dependencies (e.g., libvosk.so's dependencies) are found.
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

        // Restore original LD_LIBRARY_PATH.
        if old_ld_path.is_empty() {
            std::env::remove_var("LD_LIBRARY_PATH");
        } else {
            std::env::set_var("LD_LIBRARY_PATH", &old_ld_path);
        }

        result
    }
}


/// Opaque VOSK model (loaded from model directory).
#[repr(C)]
pub struct VoskModel {
    _private: [u8; 0],
}

/// Opaque VOSK recognizer (created from a model, processes audio).
#[repr(C)]
pub struct VoskRecognizer {
    _private: [u8; 0],
}

/// Loaded VOSK DLL with function pointers.
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
    /// Load VOSK DLL. If not found locally, auto-downloads from GitHub.
    /// Searches in: model dir, parent dir, exe dir, cwd, whisper_model/.
    pub fn load(model_dir: &Path) -> Result<Self, String> {
        // First, try to find DLL locally.
        if let Ok(dll) = Self::try_load_local(model_dir) {
            return Ok(dll);
        }

        // DLL not found — auto-download to model directory.
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

    /// Try to find and load VOSK DLL from local paths.
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

    /// Build VoskDll from an already-loaded Library.
    fn from_library(lib: Library) -> Result<Self, String> {
        // SAFETY: We load symbols by their exact C names from the VOSK API.
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

    /// Load a VOSK model from the given directory path.
    /// The directory should contain the model files (conf/, ivector/, etc.).
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

    /// Free a VOSK model.
    pub unsafe fn free_model(&self, model: *mut VoskModel) {
        if !model.is_null() {
            (self.model_free)(model);
        }
    }

    /// Create a recognizer from a loaded model.
    /// sample_rate: typically 16000.0 for 16kHz audio.
    /// Also configures SetWords(true) and SetMaxAlternatives(0) for best accuracy.
    pub unsafe fn create_recognizer(
        &self,
        model: *mut VoskModel,
        sample_rate: f32,
    ) -> Result<*mut VoskRecognizer, String> {
        let rec = (self.recognizer_new)(model, sample_rate as c_float);
        if rec.is_null() {
            return Err("vosk_recognizer_new returned NULL".into());
        }
        // Enable word-level timing info — improves recognition quality.
        (self.recognizer_set_words)(rec, 1);
        // Disable alternative hypotheses — single best result only.
        (self.recognizer_set_max_alternatives)(rec, 0);
        Ok(rec)
    }

    /// Feed audio samples to the recognizer.
    /// `samples` must be 16-bit signed integer PCM, mono, at the sample rate
    /// specified when creating the recognizer.
    /// Returns 1 if the recognizer has enough data to produce a result, 0 otherwise.
    pub unsafe fn accept_waveform(
        &self,
        rec: *mut VoskRecognizer,
        samples: &[i16],
    ) -> i32 {
        (self.recognizer_accept_waveform)(rec, samples.as_ptr(), samples.len() as c_int)
    }

    /// Get the final recognition result as a JSON string.
    /// Call after `accept_waveform` returns 1.
    /// The returned string is owned by the recognizer — copy it immediately.
    pub unsafe fn result(&self, rec: *mut VoskRecognizer) -> String {
        let ptr = (self.recognizer_result)(rec);
        if ptr.is_null() {
            return String::new();
        }
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }

    /// Free a recognizer.
    pub unsafe fn free_recognizer(&self, rec: *mut VoskRecognizer) {
        if !rec.is_null() {
            (self.recognizer_free)(rec);
        }
    }
}

/// Download VOSK DLL from GitHub releases if not present.
/// Returns the path to the downloaded DLL.
fn ensure_vosk_dll(target: &Path) -> Result<PathBuf, String> {
    if target.exists() {
        return Ok(target.to_path_buf());
    }

    // VOSK releases: https://github.com/alphacep/vosk-api/releases
    // We use a known-good release tag. The zip contains vosk.dll + dependencies.
    let release_tag = "v0.3.45";
    let asset = "vosk-win64-0.3.45.zip";
    let url = format!(
        "https://github.com/alphacep/vosk-api/releases/download/{release_tag}/{asset}"
    );

    // Download to a temp file.
    let tmp_zip = target.with_extension("zip.tmp");
    eprintln!("[VOSK] Downloading {url} ...");
    let response = reqwest::blocking::get(&url)
        .map_err(|e| format!("Download failed: {e}"))?;
    let bytes = response
        .bytes()
        .map_err(|e| format!("Read response failed: {e}"))?;
    std::fs::write(&tmp_zip, &bytes)
        .map_err(|e| format!("Write temp zip failed: {e}"))?;

    // Extract vosk.dll from the zip.
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

        // Extract vosk.dll / libvosk.dll and its dependencies.
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

    // Clean up temp zip.
    let _ = std::fs::remove_file(&tmp_zip);

    match found_dll {
        Some(path) => Ok(path),
        None => Err("vosk.dll / libvosk.dll not found in downloaded archive".into()),
    }
}
