//! FFI bindings to whisper.dll (whisper.cpp native library).
//! Uses libloading for dynamic loading — no compile-time linking required.

use std::ffi::{c_char, c_float, c_int, CStr, CString};
use std::path::Path;

use libloading::{Library, Symbol};

// Windows API: SetDllDirectoryW — adds a directory to the DLL search path.
// whisper.dll depends on ggml.dll, SDL2.dll, parakeet.dll in the same folder.
// SetDllDirectoryW ensures Windows finds them before LoadLibraryExW.
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

/// Opaque whisper context (holds loaded model in memory).
#[repr(C)]
pub struct WhisperContext {
    _private: [u8; 0],
}

/// Context initialization parameters (whisper_context_params from whisper.h).
///
/// Layout (20 bytes, confirmed via Python ctypes dump):
///   bool use_gpu;       // offset 0  (1 byte)
///   bool flash_attn;    // offset 1  (1 byte)
///   // 2 bytes padding
///   int  gpu_device;    // offset 4  (4 bytes)
///   bool dtw;           // offset 8  (1 byte)
///   // 3 bytes padding
///   int  devices;       // offset 12 (4 bytes) — bitmask, NOT a pointer
///   int  backends;      // offset 16 (4 bytes) — bitmask, NOT a pointer
/// Total: 20 bytes.
#[repr(C)]
pub struct WhisperContextParams {
    pub use_gpu: u8,       // offset 0
    pub flash_attn: u8,    // offset 1
    pub _pad1: [u8; 2],    // offset 2-3
    pub gpu_device: c_int, // offset 4-7
    pub dtw: u8,           // offset 8
    pub _pad2: [u8; 3],    // offset 9-11
    pub devices: c_int,    // offset 12-15
    pub backends: c_int,   // offset 16-19
}

/// Sampling strategies for whisper.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub enum WhisperSamplingStrategy {
    Greedy = 0,
    BeamSearch = 1,
}

/// Full parameters for whisper_full().
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

/// Loaded whisper.dll with function pointers.
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
    /// Load whisper.dll from the given directory.
    pub fn load(dll_dir: &Path) -> Result<Self, String> {
        let dll_path = dll_dir.join("whisper.dll");
        if !dll_path.exists() {
            return Err(format!("whisper.dll not found at {}", dll_path.display()));
        }

        // Add DLL directory to search path so whisper.dll can find its
        // dependencies (ggml.dll, SDL2.dll, parakeet.dll) in the same folder.
        // Must be absolute — SetDllDirectoryW requires it.
        let dll_dir_abs = dll_dir
            .canonicalize()
            .unwrap_or_else(|_| dll_dir.to_path_buf());
        add_dll_directory(&dll_dir_abs);

        // Canonicalize DLL path to absolute — libloading uses
        // LOAD_WITH_ALTERED_SEARCH_PATH for paths containing separators,
        // which tells Windows to search the DLL's directory for its
        // dependencies.
        let dll_path_abs = dll_path
            .canonicalize()
            .unwrap_or_else(|_| dll_path.clone());

        // SAFETY: libloading loads a native library. We trust whisper.dll.
        let lib = unsafe {
            Library::new(&dll_path_abs).map_err(|e| format!("Failed to load whisper.dll: {e}"))?
        };

        // SAFETY: We load symbols by their exact C names from whisper.h.
        // The function signatures must match the C ABI exactly.
        // We extract raw function pointers and drop Symbols before moving `lib`.
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

    /// Initialize whisper context from a model file with CPU-only params.
    /// The model stays loaded in memory until `free_context` is called.
    pub unsafe fn init_context(&self, model_path: &str) -> Result<*mut WhisperContext, String> {
        let c_path = CString::new(model_path)
            .map_err(|e| format!("Invalid model path: {e}"))?;

        // Build CPU-only context params.
        // use_gpu = 0 (false) — CPU only
        // backends = 1 (GGML_BACKEND_TYPE_CPU bitmask)
        // devices = 1 (at least one device must be reported)
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

    /// Free whisper context and unload model from memory.
    pub unsafe fn free_context(&self, ctx: *mut WhisperContext) {
        if !ctx.is_null() {
            (self.free)(ctx);
        }
    }

    /// Create default params for whisper_full().
    /// whisper_full_default_params_by_ref fills the struct by pointer (void return).
    pub unsafe fn default_params(&self, strategy: WhisperSamplingStrategy) -> WhisperFullParams {
        let mut params: WhisperFullParams = std::mem::zeroed();
        (self.full_default_params)(strategy as c_int, &mut params);
        params
    }

    /// Run inference on audio samples.
    /// Returns 0 on success, non-zero on error.
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

    /// Get number of segments after successful whisper_full().
    pub unsafe fn n_segments(&self, ctx: *mut WhisperContext) -> i32 {
        (self.full_n_segments)(ctx) as i32
    }

    /// Get text of a segment by index.
    pub unsafe fn segment_text(&self, ctx: *mut WhisperContext, index: i32) -> String {
        let ptr = (self.full_get_segment_text)(ctx, index as c_int);
        if ptr.is_null() {
            return String::new();
        }
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}

/// Convert i16 audio samples to f32 (required by whisper).
pub fn i16_to_f32(samples: &[i16]) -> Vec<f32> {
    samples
        .iter()
        .map(|&s| s as f32 / 32768.0)
        .collect()
}
