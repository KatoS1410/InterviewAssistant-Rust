//! Сервис распознавания речи через VOSK (FFI)

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use crossbeam_channel::{Receiver, Sender};
use parking_lot::Mutex;

use crate::core::vosk_ffi::{VoskDll, VoskModel, VoskRecognizer};

#[derive(Clone, Debug)]
pub enum TranscriptEvent {
    Final { text: String, source: String },
    Status(String),
    Error(String),
}

enum WorkerMsg {
    Load(PathBuf),
    Unload,
    Recognize { samples: Vec<i16>, source: String },
    Shutdown,
}

pub struct TranscriberService {
    cmd_tx: Sender<WorkerMsg>,
    event_tx: Arc<Mutex<Sender<TranscriptEvent>>>,
    loading: Arc<AtomicBool>,
    loaded: Arc<AtomicBool>,
    model_path: Arc<Mutex<String>>,
    worker: Option<JoinHandle<()>>,
}

impl TranscriberService {
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
        let (event_tx, _event_rx) = crossbeam_channel::unbounded();
        let event_tx = Arc::new(Mutex::new(event_tx));

        let loading = Arc::new(AtomicBool::new(false));
        let loaded = Arc::new(AtomicBool::new(false));
        let model_path = Arc::new(Mutex::new(String::new()));

        let worker_loading = Arc::clone(&loading);
        let worker_loaded = Arc::clone(&loaded);
        let worker_model_path = Arc::clone(&model_path);
        let worker_event_tx = Arc::clone(&event_tx);

        let worker = thread::Builder::new()
            .name("vosk-worker".into())
            .spawn(move || {
                vosk_worker(
                    cmd_rx,
                    worker_event_tx,
                    worker_loading,
                    worker_loaded,
                    worker_model_path,
                )
            })
            .expect("spawn vosk worker");

        Self {
            cmd_tx,
            event_tx,
            loading,
            loaded,
            model_path,
            worker: Some(worker),
        }
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded.load(Ordering::SeqCst)
    }

    pub fn is_loading(&self) -> bool {
        self.loading.load(Ordering::SeqCst)
    }

    pub fn model_path(&self) -> String {
        self.model_path.lock().clone()
    }

    pub fn load_async(&self, path: impl Into<PathBuf>, status_tx: Sender<TranscriptEvent>) {
        *self.event_tx.lock() = status_tx;
        self.loading.store(true, Ordering::SeqCst);
        let path = path.into();
        self.model_path
            .lock()
            .clone_from(&path.display().to_string());
        let _ = self.cmd_tx.send(WorkerMsg::Load(path));
    }

    pub fn reload_async(&self, path: Option<PathBuf>, status_tx: Sender<TranscriptEvent>) {
        self.unload();
        self.load_async(
            path.unwrap_or_else(|| PathBuf::from(self.model_path())),
            status_tx,
        );
    }

    pub fn unload(&self) {
        let _ = self.cmd_tx.send(WorkerMsg::Unload);
    }

    /// Распознать весь буфер (вызывается один раз после остановки записи)
    pub fn recognize(&self, samples: &[i16], source: &str) {
        if !self.is_loaded() || samples.is_empty() {
            return;
        }
        let _ = self.cmd_tx.send(WorkerMsg::Recognize {
            samples: samples.to_vec(),
            source: source.into(),
        });
    }
}

impl Drop for TranscriberService {
    fn drop(&mut self) {
        let _ = self.cmd_tx.send(WorkerMsg::Shutdown);
        if let Some(worker) = self.worker.take() {
            // Корректное завершение с таймаутом 3 секунды
            let (done_tx, done_rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let _ = worker.join();
                let _ = done_tx.send(());
            });
            let _ = done_rx.recv_timeout(std::time::Duration::from_secs(3));
        }
    }
}

impl Default for TranscriberService {
    fn default() -> Self {
        Self::new()
    }
}

pub const SAMPLE_RATE: u32 = 16000;

fn vosk_worker(
    cmd_rx: Receiver<WorkerMsg>,
    event_tx: Arc<Mutex<Sender<TranscriptEvent>>>,
    loading: Arc<AtomicBool>,
    loaded: Arc<AtomicBool>,
    model_path: Arc<Mutex<String>>,
) {
    let mut dll: Option<VoskDll> = None;
    let mut model: *mut VoskModel = std::ptr::null_mut();

    let send = |event_tx: &Arc<Mutex<Sender<TranscriptEvent>>>, event: TranscriptEvent| {
        let tx = event_tx.lock().clone();
        let _ = tx.send(event);
    };

    while let Ok(msg) = cmd_rx.recv() {
        match msg {
            WorkerMsg::Load(path) => {
                send(
                    &event_tx,
                    TranscriptEvent::Status(format!("Loading VOSK model ({})", path.display())),
                );

                if !path.exists() || !path.is_dir() {
                    send(
                        &event_tx,
                        TranscriptEvent::Error(format!(
                            "VOSK model directory not found: {}",
                            path.display()
                        )),
                    );
                    loaded.store(false, Ordering::SeqCst);
                    loading.store(false, Ordering::SeqCst);
                    continue;
                }

                // Освобождаем предыдущую модель
                if !model.is_null() {
                    if let Some(ref d) = dll {
                        unsafe { d.free_model(model); }
                    }
                    model = std::ptr::null_mut();
                }

                // Загружаем VOSK DLL (скачивает если нужно)
                if dll.is_none() {
                    send(
                        &event_tx,
                        TranscriptEvent::Status("Loading VOSK DLL (may download)...".into()),
                    );
                    match VoskDll::load(&path) {
                        Ok(d) => dll = Some(d),
                        Err(e) => {
                            send(&event_tx, TranscriptEvent::Error(e));
                            loaded.store(false, Ordering::SeqCst);
                            loading.store(false, Ordering::SeqCst);
                            continue;
                        }
                    }
                }

                let dll_ref = dll.as_ref().unwrap();

                // Загружаем модель VOSK
                let model_dir_str = path.display().to_string();
                match unsafe { dll_ref.load_model(&model_dir_str) } {
                    Ok(m) => {
                        model = m;
                        model_path.lock().clone_from(&model_dir_str);
                        loaded.store(true, Ordering::SeqCst);
                        loading.store(false, Ordering::SeqCst);
                        send(&event_tx, TranscriptEvent::Status("VOSK ready".into()));
                    }
                    Err(e) => {
                        send(&event_tx, TranscriptEvent::Error(e));
                        loaded.store(false, Ordering::SeqCst);
                        loading.store(false, Ordering::SeqCst);
                    }
                }
            }
            WorkerMsg::Unload => {
                if !model.is_null() {
                    if let Some(ref d) = dll {
                        unsafe { d.free_model(model); }
                    }
                    model = std::ptr::null_mut();
                }
                loaded.store(false, Ordering::SeqCst);
                loading.store(false, Ordering::SeqCst);
                model_path.lock().clear();
            }
            WorkerMsg::Recognize { samples, source } => {
                if model.is_null() || dll.is_none() {
                    send(
                        &event_tx,
                        TranscriptEvent::Error("VOSK model not loaded".into()),
                    );
                    continue;
                }

                let dll_ref = dll.as_ref().unwrap();

                // Новый распознаватель для этой сессии
                let recognizer: *mut VoskRecognizer = match unsafe {
                    dll_ref.create_recognizer(model, SAMPLE_RATE as f32)
                } {
                    Ok(rec) => rec,
                    Err(e) => {
                        send(&event_tx, TranscriptEvent::Error(e));
                        continue;
                    }
                };

                send(
                    &event_tx,
                    TranscriptEvent::Status("Transcribing...".into()),
                );

                // Отправляем все образцы за один проход
                unsafe {
                    dll_ref.accept_waveform(recognizer, &samples);
                }

                // Получаем финальный результат
                let result = unsafe { dll_ref.result(recognizer) };
                let text = parse_vosk_json(&result);

                // Освобождаем распознаватель сессии
                unsafe { dll_ref.free_recognizer(recognizer); }

                if !text.is_empty() {
                    send(
                        &event_tx,
                        TranscriptEvent::Final {
                            text,
                            source,
                        },
                    );
                }
            }
            WorkerMsg::Shutdown => {
                if !model.is_null() {
                    if let Some(ref d) = dll {
                        unsafe { d.free_model(model); }
                    }
                }
                break;
            }
        }
    }
}

/// Извлечение текста из JSON результата VOSK
fn parse_vosk_json(json: &str) -> String {
    if json.is_empty() {
        return String::new();
    }

    // Поле "text" (финальный результат)
    if let Some(start) = json.find("\"text\"") {
        if let Some(colon) = json[start..].find(':') {
            let after_colon = &json[start + colon + 1..];
            if let Some(open) = after_colon.find('"') {
                let content = &after_colon[open + 1..];
                if let Some(close) = content.find('"') {
                    return content[..close].to_string();
                }
            }
        }
    }

    // Поле "partial" (промежуточный результат)
    if let Some(start) = json.find("\"partial\"") {
        if let Some(colon) = json[start..].find(':') {
            let after_colon = &json[start + colon + 1..];
            if let Some(open) = after_colon.find('"') {
                let content = &after_colon[open + 1..];
                if let Some(close) = content.find('"') {
                    return content[..close].to_string();
                }
            }
        }
    }

    String::new()
}
