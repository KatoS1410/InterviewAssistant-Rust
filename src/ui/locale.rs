//! Localization strings for the UI.
//! Keys are grouped by tab/context.

#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Lang {
    #[serde(rename = "ru")]
    Ru,
    #[serde(rename = "en")]
    En,
}

impl Lang {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "en" | "english" => Lang::En,
            _ => Lang::Ru,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Lang::Ru => "ru",
            Lang::En => "en",
        }
    }

}

/// Returns the localized string for the given key.
/// Falls back to Russian if key is not found.
pub fn t(lang: Lang, key: &str) -> &'static str {
    let entry = LOCALE.get(key).unwrap_or(&("???", "???"));
    match lang {
        Lang::Ru => entry.0,
        Lang::En => entry.1,
    }
}

/// All translatable strings: key → (ru, en)
static LOCALE: phf::Map<&'static str, (&'static str, &'static str)> = phf::phf_map! {
    // --- Tabs ---
    "tab.main" => ("Основное", "Main"),
    "tab.history" => ("История", "History"),
    "tab.settings" => ("Настройки", "Settings"),
    "tab.logs" => ("Логи", "Logs"),

    // --- Header ---
    "header.title" => ("Interview Assistant", "Interview Assistant"),
    "header.hint" => ("[<-] loopback  [->] mic", "[<-] loopback  [->] mic"),

    // --- Main tab ---
    "main.transcript" => ("Транскрипт", "Transcript"),
    "main.answer" => ("Ответ AI", "AI Answer"),
    "main.ask" => ("Спросить AI", "Ask AI"),
    "main.clear" => ("Очистить", "Clear"),
    "main.copy_answer" => ("Копировать ответ", "Copy Answer"),
    "main.copy_question" => ("Копировать вопрос", "Copy Question"),
    "main.prev_question" => ("Предыдущий вопрос", "Previous Question"),
    "main.prev_answer" => ("Предыдущий ответ", "Previous Answer"),
    "main.thinking" => ("Думаю...", "Thinking..."),
    "main.no_answer" => ("Нет ответа от ИИ, проверьте доступность модели", "No AI response, check model availability"),
    "main.vosk_not_loaded" => ("VOSK не подключен! Распознавание невозможно.", "VOSK not loaded! Recognition unavailable."),
    "main.recording" => ("Запись...", "Recording..."),
    "main.stopping" => ("Остановка записи... (сбор хвоста)", "Stopping... (collecting tail)"),
    "main.stopped" => ("Запись остановлена, распознавание...", "Recording stopped, recognizing..."),
    "main.source_loopback" => ("Источник: LOOPBACK", "Source: LOOPBACK"),
    "main.source_mic" => ("Источник: MIC", "Source: MIC"),
    "main.live_speech" => ("Live speech stream", "Live speech stream"),
    "main.ai_error" => ("Ошибка AI", "AI Error"),
    "main.ai_timeout" => ("Таймаут AI", "AI Timeout"),
    "main.ai_response" => ("Ответ получен", "Response received"),
    "main.ai_request" => ("Запрос к AI...", "AI request..."),
    "main.no_text" => ("Нет текста для AI", "No text for AI"),

    // --- History tab ---
    "history.title" => ("История вопросов и ответов", "Question & Answer History"),
    "history.questions" => ("Вопросы", "Questions"),
    "history.answers" => ("Ответы", "Answers"),
    "history.clear" => ("Очистить историю", "Clear History"),
    "history.empty" => ("История пуста", "History is empty"),

    // --- Settings tab ---
    "settings.ai_provider" => ("AI / Provider", "AI / Provider"),
    "settings.provider" => ("Provider", "Provider"),
    "settings.auth_key" => ("Authorization Key", "Authorization Key"),
    "settings.scope" => ("Scope", "Scope"),
    "settings.address" => ("Address", "Address"),
    "settings.port" => ("Port", "Port"),
    "settings.api_key" => ("API Key", "API Key"),
    "settings.base_url" => ("Base URL", "Base URL"),
    "settings.model" => ("Model", "Model"),
    "settings.test" => ("Проверить", "Test"),
    "settings.position" => ("Должность", "Position"),
    "settings.system_prompt" => ("System Prompt", "System Prompt"),
    "settings.vosk_audio" => ("VOSK / Аудио", "VOSK / Audio"),
    "settings.model_path" => ("Model path", "Model path"),
    "settings.chunk_ms" => ("Chunk ms", "Chunk ms"),
    "settings.auto_ask_sec" => ("Auto ask sec", "Auto ask sec"),
    "settings.tail_ms" => ("Tail ms", "Tail ms"),
    "settings.tail_hint" => ("Задержка после остановки для сбора хвоста loopback", "Delay after stop to collect loopback tail"),
    "settings.load_vosk" => ("Загрузить VOSK", "Load VOSK"),
    "settings.reload_vosk" => ("Перезагрузить", "Reload"),
    "settings.download_hint" => ("Скачать VOSK модель можно здесь:", "Download VOSK model here:"),
    "settings.unpack_hint" => ("Распакуйте архив и укажите путь к папке с моделью.", "Extract the archive and set the model folder path."),
    "settings.dll_hint" => ("DLL скачается автоматически при загрузке модели.", "DLL downloads automatically when loading the model."),
    "settings.vosk_status" => ("VOSK:", "VOSK:"),
    "settings.show_config" => ("Показать конфиг для редактирования", "Show config for editing"),
    "settings.hide_config" => ("Скрыть конфиг", "Hide config"),
    "settings.config_title" => ("Конфиг", "Config"),
    "settings.save" => ("Сохранить", "Save"),
    "settings.export" => ("Экспорт", "Export"),
    "settings.import" => ("Импорт", "Import"),
    "settings.language" => ("Язык", "Language"),
    "settings.saved" => ("Настройки сохранены", "Settings saved"),

    // --- Logs tab ---
    "logs.title" => ("Логи приложения", "Application Logs"),
    "logs.clear" => ("Очистить логи", "Clear Logs"),

    // --- Status bar ---
    "status.devices" => ("Устройств:", "Devices:"),
    "status.devices_count" => ("Устройств: {}", "Devices: {}"),
    "status.recording_blocked" => ("Ошибка: VOSK не загружен", "Error: VOSK not loaded"),
    "status.ai" => ("AI:", "AI:"),
    "status.vosk" => ("VOSK:", "VOSK:"),
    "status.hz" => ("Hz", "Hz"),

    // --- VOSK status ---
    "vosk.not_loaded" => ("not loaded", "not loaded"),
    "vosk.loading" => ("loading...", "loading..."),
    "vosk.ready" => ("ready", "ready"),
    "vosk.error" => ("error", "error"),
    "vosk.no_path" => ("no model path", "no model path"),

    // --- Misc ---
    "misc.browse" => ("...", "..."),
    "misc.loopback_found" => ("Loopback найден", "Loopback found"),
    "misc.loopback_not_found" => ("Loopback не найден", "Loopback not found"),
    "misc.mic_found" => ("Микрофон найден", "Microphone found"),
    "misc.mic_not_found" => ("Микрофон не найден", "Microphone not found"),
    "misc.app_initialized" => ("App initialized", "App initialized"),
    "misc.config_saved" => ("Config saved", "Config saved"),
    "misc.recording_blocked" => ("Recording blocked: VOSK not loaded", "Recording blocked: VOSK not loaded"),
};
