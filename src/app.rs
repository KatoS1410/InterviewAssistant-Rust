use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender, unbounded};
use eframe::egui;

use crate::config::{self, AppConfig};
use crate::core::{
    list_input_devices, timestamp, to_int, SingleInstanceGuard,
};
use crate::services::{
    ai::{spawn_ai_request, AiEvent, AiSession},
    audio::{AudioMode, AudioRecorder, SAMPLE_RATE},
    hotkeys::{HotkeyAction, HotkeyService},
    transcriber::{TranscriberService, TranscriptEvent},
};
use crate::ui::theme::{apply_theme, draw_header, Theme};
use egui::Color32;

pub struct InterviewApp {
    pub cfg: AppConfig,
    pub tab: usize,

    pub transcript: String,
    pub question: String,
    pub prev_question: String,
    pub answer: String,
    pub prev_answer: String,
    pub last_error: String,
    pub logs: String,
    pub history_questions: String,
    pub history_answers: String,
    pub status: String,
    pub transcript_hint: String,
    pub vosk_status: String,
    pub config_preview: String,
    pub chunk_ms_text: String,
    pub auto_ask_text: String,

    pub device_names: Vec<String>,
    pub recording: bool,
    pub record_mode: Option<AudioMode>,
    pub active_hold: Option<HotkeySide>,

    audio: AudioRecorder,
    transcriber: TranscriberService,
    ai: Arc<Mutex<AiSession>>,
    audio_buffer: Vec<i16>,

    audio_tx: Sender<Vec<i16>>,
    audio_rx: Receiver<Vec<i16>>,
    transcript_tx: Sender<TranscriptEvent>,
    transcript_rx: Receiver<TranscriptEvent>,
    ai_tx: Sender<AiEvent>,
    ai_rx: Receiver<AiEvent>,
    hotkey_rx: Receiver<HotkeyAction>,

    _hotkeys: Option<HotkeyService>,
    _instance: Option<SingleInstanceGuard>,
    auto_ask_deadline: Option<Instant>,
    awaiting_transcript: bool,
    ai_busy: bool,
    /// True when stop was requested but the capture thread is still
    /// collecting the loopback tail (non-blocking stop).
    stopping: bool,

    // UI state
    pub config_edit_mode: bool,
    pub ai_request_time: Option<Instant>,
    pub big_status: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HotkeySide {
    Left,
    Right,
}

impl InterviewApp {
    pub fn new(instance: SingleInstanceGuard) -> Self {
        let cfg = config::load();
        let chunk_ms_text = cfg.chunk_ms.to_string();
        let auto_ask_text = cfg.auto_ask_sec.to_string();
        let (audio_tx, audio_rx) = unbounded();
        let (transcript_tx, transcript_rx) = unbounded();
        let (ai_tx, ai_rx) = unbounded();
        let (hotkey_tx, hotkey_rx) = unbounded();

        let ai = Arc::new(Mutex::new(AiSession::new(cfg.clone())));
        let transcriber = TranscriberService::new();
        let hotkeys = HotkeyService::install(hotkey_tx).ok();

        let mut app = Self {
            cfg: cfg.clone(),
            tab: 0,
            transcript: String::new(),
            question: String::new(),
            prev_question: String::new(),
            answer: String::new(),
            prev_answer: String::new(),
            last_error: String::new(),
            logs: String::new(),
            history_questions: String::new(),
            history_answers: String::new(),
            status: "[<-] loopback  [->] mic".into(),
            transcript_hint: "Live speech stream".into(),
            vosk_status: "not loaded".into(),
            config_preview: serde_json::to_string_pretty(&cfg).unwrap_or_default(),
            chunk_ms_text,
            auto_ask_text,
            device_names: Vec::new(),
            recording: false,
            record_mode: None,
            active_hold: None,

            audio: AudioRecorder::new(),
            transcriber,
            ai,
            audio_buffer: Vec::new(),
            audio_tx,
            audio_rx,
            transcript_tx,
            transcript_rx,
            ai_tx,
            ai_rx,
            hotkey_rx,
            _hotkeys: hotkeys,
            _instance: Some(instance),
            auto_ask_deadline: None,
            awaiting_transcript: false,
            ai_busy: false,
            stopping: false,
            config_edit_mode: false,
            ai_request_time: None,
            big_status: String::new(),
        };

        app.refresh_devices();
        app.log("App initialized");
        app.load_vosk();
        app
    }

    pub fn log(&mut self, msg: &str) {
        let line = format!("[{}] {msg}\n", timestamp());
        self.logs.push_str(&line);
    }

    pub fn set_status(&mut self, msg: &str) {
        self.status = msg.to_string();
    }

    pub fn refresh_devices(&mut self) {
        self.device_names = list_input_devices()
            .into_iter()
            .map(|d| d.name)
            .collect();
        if self.cfg.loopback_device.is_empty() {
            if let Some(dev) = crate::core::find_loopback_device() {
                self.cfg.loopback_device = dev.name;
            }
        }
        if self.cfg.mic_device.is_empty() {
            if let Some(dev) = crate::core::find_mic_device() {
                self.cfg.mic_device = dev.name;
            }
        }
        self.set_status(&format!("Устройств: {}", self.device_names.len()));
        self.log(&format!("Devices refreshed: {}", self.device_names.len()));
        self.refresh_config_preview();
    }

    pub fn detect_loopback(&mut self) {
        if let Some(dev) = crate::core::find_loopback_device() {
            self.cfg.loopback_device = dev.name.clone();
            self.set_status("Loopback найден");
            self.log(&format!("Loopback: {}", dev.name));
        } else {
            self.set_status("Loopback не найден");
            self.log("Loopback not found");
        }
    }

    pub fn detect_mic(&mut self) {
        if let Some(dev) = crate::core::find_mic_device() {
            self.cfg.mic_device = dev.name.clone();
            self.set_status("Микрофон найден");
            self.log(&format!("Mic: {}", dev.name));
        } else {
            self.set_status("Микрофон не найден");
            self.log("Mic not found");
        }
    }

    pub fn save_config(&mut self) {
        self.sync_numeric_fields();
        self.auto_ask_deadline = None;
        if let Err(err) = config::save(&self.cfg) {
            self.log(&format!("Save error: {err}"));
        } else {
            self.set_status("Настройки сохранены");
            self.log("Config saved");
            self.ai.lock().unwrap().configure(self.cfg.clone());
        }
        self.refresh_config_preview();
    }

    pub fn export_config(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name("config.json")
            .save_file()
        {
            self.sync_numeric_fields();
            match config::export_to(&path, &self.cfg) {
                Ok(_) => self.log(&format!("Exported: {}", path.display())),
                Err(err) => self.log(&format!("Export error: {err}")),
            }
        }
    }

    pub fn import_config(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            match config::import_from(&path) {
                Ok(cfg) => {
                    self.cfg = cfg;
                    self.chunk_ms_text = self.cfg.chunk_ms.to_string();
                    self.auto_ask_text = self.cfg.auto_ask_sec.to_string();
                    self.refresh_config_preview();
                    self.log(&format!("Imported: {}", path.display()));
                }
                Err(err) => self.log(&format!("Import error: {err}")),
            }
        }
    }

    pub fn refresh_config_preview(&mut self) {
        self.config_preview = serde_json::to_string_pretty(&self.cfg).unwrap_or_default();
    }

    fn sync_numeric_fields(&mut self) {
        self.cfg.chunk_ms = to_int(&self.chunk_ms_text, 250).max(20) as u32;
        self.cfg.auto_ask_sec = to_int(&self.auto_ask_text, 0).max(0) as u32;
    }

    /// Загрузка VOSK модели из указанного в конфиге пути.
    pub fn load_vosk(&mut self) {
        let path = self.cfg.vosk_model_path.clone();
        if path.is_empty() {
            self.vosk_status = "no model path".into();
            self.log("VOSK: no model path configured");
            return;
        }
        if self.transcriber.is_loading() {
            self.log("VOSK: already loading, skipping");
            return;
        }
        if self.transcriber.is_loaded() {
            self.log("VOSK: already loaded, skipping");
            return;
        }
        let tx = self.transcript_tx.clone();
        self.vosk_status = "loading...".into();
        self.transcriber.load_async(PathBuf::from(&path), tx);
        self.log(&format!("VOSK load started: {path}"));
    }

    /// Выбор папки с VOSK моделью через диалог (только устанавливает путь).
    pub fn browse_vosk_model(&mut self) {
        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
            self.cfg.vosk_model_path = dir.display().to_string();
            self.refresh_config_preview();
            self.save_config();
            self.log(&format!("VOSK model path set: {}", self.cfg.vosk_model_path));
        }
    }

    pub fn reload_vosk(&mut self) {
        self.save_config();
        let path = PathBuf::from(self.cfg.vosk_model_path.clone());
        let tx = self.transcript_tx.clone();
        self.transcriber.reload_async(Some(path), tx);
        self.log("VOSK reload");
    }

    pub fn test_ai(&mut self) {
        self.save_config();
        let result = self.ai.lock().unwrap().test_connection();
        self.log(&format!(
            "AI test: {} — {}",
            if result.ok { "OK" } else { "FAIL" },
            result.message
        ));
        self.set_status(&result.message);
    }

    pub fn ask_ai(&mut self) {
        if self.ai_busy {
            return;
        }
        let q = self.question.trim().to_string();
        if q.is_empty() {
            self.set_status("Нет текста для AI");
            return;
        }
        // Сохраняем предыдущий вопрос и ответ для кнопок истории.
        if self.prev_question != q {
            self.prev_question = self.question.clone();
        }
        if !self.answer.is_empty() && self.answer != "Думаю..." {
            self.prev_answer = self.answer.clone();
        }
        self.save_config();
        self.answer = "Думаю...".into();
        self.ai_busy = true;
        self.ai_request_time = Some(Instant::now());
        spawn_ai_request(
            self.cfg.clone(),
            Arc::clone(&self.ai),
            q,
            self.ai_tx.clone(),
        );
        self.set_status("Запрос к AI...");
        self.log("AI request started");
    }

    fn start_recording(&mut self, mode: AudioMode) {
        if self.recording {
            return;
        }
        // Проверяем, что VOSK загружен.
        if !self.transcriber.is_loaded() && !self.transcriber.is_loading() {
            self.big_status = "VOSK не подключен! Распознавание невозможно.".into();
            self.set_status("Ошибка: VOSK не загружен");
            self.log("Recording blocked: VOSK not loaded");
            return;
        }
        self.save_config();
        self.sync_numeric_fields();
        self.audio_buffer.clear();
        // Clear transcript for new question (C# approach: replace, don't accumulate).
        self.transcript.clear();

        let device = match mode {
            AudioMode::Loopback => self.cfg.loopback_device.clone(),
            AudioMode::Mic => self.cfg.mic_device.clone(),
        };

        match self.audio.start(
            mode,
            &device,
            self.cfg.chunk_ms,
            self.audio_tx.clone(),
        ) {
            Ok(_) => {
                self.recording = true;
                self.record_mode = Some(mode);
                self.transcript_hint = match mode {
                    AudioMode::Loopback => "Источник: LOOPBACK".into(),
                    AudioMode::Mic => "Источник: MIC".into(),
                };
                self.set_status("Запись...");
                self.log(&format!("Recording started: {mode:?} ({device})"));
            }
            Err(err) => {
                self.log(&format!("Audio error: {err}"));
                self.set_status(&err.to_string());
            }
        }
    }

    fn stop_recording(&mut self) {
        if !self.recording {
            return;
        }
        // Non-blocking: signal the capture thread to stop.
        // The thread will continue for TAIL_MS to collect the loopback tail.
        // Drain + recognize happens in poll_channels() when is_active() becomes false.
        self.audio.stop();
        self.recording = false;
        self.stopping = true;

        self.set_status("Остановка записи... (сбор хвоста)");
        self.log("Recording stop signalled");
    }

    fn apply_transcript_event(&mut self, event: TranscriptEvent) {
        match event {
            TranscriptEvent::Final { text, source } => {
                let line = format!("[{source}] {text}");
                if !self.transcript.is_empty() && !self.transcript.ends_with('\n') {
                    self.transcript.push('\n');
                }
                self.transcript.push_str(&line);
                self.big_status.clear();
                if self.awaiting_transcript {
                    self.awaiting_transcript = false;
                    self.question = self.transcript.clone();
                    self.ask_ai();
                }
            }
            TranscriptEvent::Status(msg) => {
                self.vosk_status = msg.clone();
                self.log(&msg);
            }
            TranscriptEvent::Error(msg) => {
                self.vosk_status = "error".into();
                self.big_status.clear();
                self.log(&msg);
            }
        }
    }

    fn poll_channels(&mut self) {
        // --- Non-blocking stop completion ---
        // When the capture thread finishes (after TAIL_MS), drain remaining
        // chunks and feed the entire buffer to VOSK.
        if self.stopping && !self.audio.is_active() {
            self.stopping = false;

            let source = match self.record_mode {
                Some(AudioMode::Loopback) => "LOOPBACK",
                Some(AudioMode::Mic) => "MIC",
                None => "AUDIO",
            };

            // Drain all remaining audio chunks into our buffer.
            while let Ok(chunk) = self.audio_rx.try_recv() {
                self.audio_buffer.extend_from_slice(&chunk);
            }

            // Feed the entire buffer to VOSK in one pass (C# approach).
            if !self.audio_buffer.is_empty() {
                self.transcriber.recognize(&self.audio_buffer, source);
            }
            self.audio_buffer.clear();

            self.record_mode = None;
            self.transcript_hint = "Live speech stream".into();
            self.big_status = "Запись остановлена, распознавание...".into();
            self.set_status("Запись остановлена, распознавание...");
            self.log("Recording stopped (tail collected)");

            // Wait for final transcript, then auto-ask AI.
            self.awaiting_transcript = true;
        }

        // Accumulate audio chunks into buffer during recording.
        // No streaming to VOSK — entire buffer is fed at stop (C# approach).
        while let Ok(chunk) = self.audio_rx.try_recv() {
            self.audio_buffer.extend_from_slice(&chunk);
        }

        while let Ok(event) = self.transcript_rx.try_recv() {
            self.apply_transcript_event(event);
        }
        if self.transcriber.is_loaded() && !self.transcriber.is_loading() && self.vosk_status != "ready" {
            self.vosk_status = "ready".into();
        }

        while let Ok(event) = self.ai_rx.try_recv() {
            match event {
                AiEvent::Answer(text) => {
                    // Сохраняем в историю
                    if !self.question.is_empty() {
                        if !self.history_questions.is_empty() {
                            self.history_questions.push_str("\n\n");
                        }
                        self.history_questions.push_str(&self.question);
                    }
                    if !text.is_empty() {
                        if !self.history_answers.is_empty() {
                            self.history_answers.push_str("\n\n");
                        }
                        self.history_answers.push_str(&text);
                    }
                    self.answer = text;
                    self.big_status.clear();
                    self.set_status("Ответ получен");
                    self.log("AI request completed");
                }
                AiEvent::Error(err) => {
                    // Сохраняем неотвеченный вопрос в историю с пометкой ошибки
                    if !self.question.is_empty() {
                        if !self.history_questions.is_empty() {
                            self.history_questions.push_str("\n\n");
                        }
                        self.history_questions.push_str(&self.question);
                    }
                    if !self.history_answers.is_empty() {
                        self.history_answers.push_str("\n\n");
                    }
                    self.history_answers.push_str(&format!("[ОШИБКА] {err}"));
                    // НЕ затираем answer — сохраняем последний успешный ответ.
                    // Ошибку показываем в last_error (под правым окном).
                    self.last_error = format!("Ошибка AI: {err}");
                    self.big_status.clear();
                    self.set_status("Ошибка AI");
                    self.log(&format!("AI error: {err}"));
                }
            }
            self.ai_busy = false;
            self.ai_request_time = None;
        }

        // Таймаут: если нет ответа 5 секунд — показываем предупреждение.
        if self.ai_busy {
            if let Some(start) = self.ai_request_time {
                if start.elapsed() >= Duration::from_secs(5) {
                    self.big_status =
                        "Нет ответа от ИИ, проверьте доступность модели".into();
                    self.set_status("Таймаут AI");
                }
            }
        }

        while let Ok(action) = self.hotkey_rx.try_recv() {
            match action {
                HotkeyAction::LoopbackPress if self.active_hold.is_none() => {
                    self.active_hold = Some(HotkeySide::Left);
                    self.start_recording(AudioMode::Loopback);
                }
                HotkeyAction::LoopbackRelease
                    if self.active_hold == Some(HotkeySide::Left) =>
                {
                    self.active_hold = None;
                    if self.record_mode == Some(AudioMode::Loopback) {
                        self.stop_recording();
                    }
                }
                HotkeyAction::MicPress if self.active_hold.is_none() => {
                    self.active_hold = Some(HotkeySide::Right);
                    self.start_recording(AudioMode::Mic);
                }
                HotkeyAction::MicRelease if self.active_hold == Some(HotkeySide::Right) => {
                    self.active_hold = None;
                    if self.record_mode == Some(AudioMode::Mic) {
                        self.stop_recording();
                    }
                }
                _ => {}
            }
        }

    }
}

impl eframe::App for InterviewApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_channels();
        ctx.request_repaint_after(Duration::from_millis(50));

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Theme::BG))
            .show(ctx, |ui| {
                draw_header(
                    ui,
                    "Interview Assistant",
                    "[<-] loopback  [->] mic",
                    self.recording,
                );
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    for (idx, name) in ["Основное", "История", "Настройки", "Логи"].iter().enumerate() {
                        let selected = self.tab == idx;
                        let (fill, text_color, stroke_color) = if selected {
                            (Theme::ACCENT_SOFT, Color32::WHITE, Theme::ACCENT)
                        } else {
                            (Theme::TAB_INACTIVE, Theme::TEXT, Theme::BORDER)
                        };
                        let button = egui::Button::new(
                            egui::RichText::new(*name).color(text_color).strong(),
                        )
                        .fill(fill)
                        .stroke(egui::Stroke::new(1.5, stroke_color))
                        .rounding(egui::Rounding::same(10.0));
                        if ui.add(button).clicked() {
                            self.tab = idx;
                        }
                    }
                });

                ui.add_space(8.0);

                match self.tab {
                    0 => crate::ui::main_tab::show(ui, self),
                    1 => crate::ui::history_tab::show(ui, self),
                    2 => crate::ui::settings_tab::show(ui, self),
                    _ => crate::ui::logs_tab::show(ui, self),
                }

                ui.add_space(8.0);
                crate::ui::status_bar(
                    ui,
                    &self.status,
                    &format!("AI: {}", self.cfg.model),
                    &format!(
                        "VOSK: {} | {} Hz",
                        self.vosk_status,
                        SAMPLE_RATE
                    ),
                );
            });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.audio.stop();
    }
}

pub fn launch() -> eframe::Result<()> {
    let instance = match crate::core::acquire_single_instance() {
        Some(guard) => guard,
        None => {
            eprintln!("Interview Assistant is already running.");
            std::process::exit(0);
        }
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([960.0, 640.0])
            .with_title("Interview Assistant"),
        ..Default::default()
    };

    eframe::run_native(
        "Interview Assistant",
        options,
        Box::new(|cc| {
            apply_theme(&cc.egui_ctx);
            Ok(Box::new(InterviewApp::new(instance)))
        }),
    )
}