# Interview Assistant (Rust + egui + VOSK)

[English](#english) | [Русский](#русский)

---

## English

A desktop assistant for technical interviews: **offline speech recognition (VOSK) + LLM (OpenAI / DeepSeek / OpenRouter / Ollama / GigaChat / custom endpoint)**.

Hold a hotkey, speak, release — the app transcribes your speech and sends it to the AI. The answer appears in the right panel.

### Features

- **Offline STT** — VOSK speech recognition, no internet required for transcription
- **Multiple AI providers** — OpenAI, DeepSeek, OpenRouter, Ollama (local), GigaChat, custom OpenAI-compatible endpoints
- **Global hotkeys** — `←` (loopback/system audio) and `→` (microphone), works even when the window is not focused
- **Non-blocking UI** — capture and recognition run in background threads, the UI stays responsive
- **Dark glass theme** — iOS-inspired design. I don't know your preferences, but iOS looks cool for me. 
- **History tab** — all Q&A pairs saved for review
- **Configurable** — JSON config file, editable in-app or manually

### Quick Start (Windows)

**Option A — Prebuilt .exe:** download from [Releases](https://github.com/KatoS1410/InterviewAssistant-Python/releases/tag/v.1.1-rust), run, configure VOSK model path in Settings.

**Option B — Build from source:**
1. **Install Rust** (if not already): https://rustup.rs
2. **Download a VOSK model**:
   - Russian: [vosk-model-ru-0.10](https://alphacephei.com/vosk/models/vosk-model-ru-0.10.zip) (~1.4 GB)
   - English: [vosk-model-en-us-0.22](https://alphacephei.com/vosk/models/vosk-model-en-us-0.22.zip) (~1.8 GB)
   - More models: https://alphacephei.com/vosk/models
3. **Extract** the model to a folder (e.g., `C:\vosk\vosk-model-ru-0.10`)
4. **Build & run**:
   ```bat
   cargo build --release
   .\target\release\interview-assistant.exe
   ```
5. **First launch**: go to Settings tab → click "Browse" next to VOSK model path → select the model folder → click "Load VOSK" (DLL downloads automatically)
6. **Configure AI**: in Settings, set your API key, model, and endpoint
7. **Set up audio devices**: click "Detect Loopback" and "Detect Mic", or select manually

### Quick Start (Linux — Debian 12+ / Ubuntu 22.04+)

**Option A — One-liner installer:**
```bash
curl -fsSL https://raw.githubusercontent.com/KatoS1410/InterviewAssistant-Python/RustConversion/linux/install.sh | bash
```
This installs system dependencies, Rust (if needed), clones the repo, builds the binary, and creates a desktop entry.

**Option B — .deb package:**
```bash
# Download the .deb from Releases, then:
sudo dpkg -i interview-assistant_1.0.1_amd64.deb
sudo apt-get install -f   # auto-install missing dependencies
```

**Option C — Manual build:**
```bash
# Install system dependencies first:
sudo apt-get install -y build-essential pkg-config libasound2-dev libgtk-3-dev libx11-dev libxcb1-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libwayland-dev curl git cmake
# Then build:
git clone -b RustConversion https://github.com/KatoS1410/InterviewAssistant-Python.git
cd InterviewAssistant-Python
cargo build --release
sudo cp target/release/interview-assistant /usr/local/bin/
```

After installation, download a VOSK model, extract it, set the path in Settings, and click "Load VOSK" — the DLL downloads automatically.

### Hotkeys

| Action | Key |
|---|---|
| Record system audio (loopback) | `←` hold |
| Record microphone | `→` hold |
| Stop + transcribe + ask AI | release the key |

### Audio Setup

**Loopback** (capture system audio — e.g., interviewer's voice from a call):
- Windows: enable "Stereo Mix" in Sound settings, or install [VB-Cable](https://vb-audio.com/Cable/)
- The app auto-detects loopback devices

**Microphone**: any standard mic, auto-detected

### AI Providers

| Provider | Endpoint | Notes |
|---|---|---|
| OpenAI | `https://api.openai.com/v1` | Requires API key |
| DeepSeek | `https://api.deepseek.com/v1` | Requires API key |
| OpenRouter | `https://openrouter.ai/api/v1` | Requires API key |
| Ollama | `http://localhost:11434/v1` | Free, local, no key needed |
| GigaChat | `https://gigachat.devices.sberbank.ru/api/v1` | Requires Sber API key |
| Custom | Any OpenAI-compatible URL | Bring your own endpoint |

### Configuration

Config file location: `%APPDATA%\katos_interview_assistant\config.json` (Windows) or `~/.katos_interview_assistant/config.json` (Linux/macOS)

Key settings:
- `model` — AI model name (e.g., `gpt-4o`, `deepseek-chat`, `llama3`)
- `api_key` — your API key
- `endpoint` — provider URL
- `vosk_model_path` — path to extracted VOSK model folder
- `loopback_device` / `mic_device` — audio device names
- `chunk_ms` — audio chunk size in ms (default: 500)
- `auto_ask_sec` — auto-send to AI after N seconds of silence (0 = disabled)

### Build from Source

```bash
# Requirements: Rust 1.75+, Git
git clone -b RustConversion https://github.com/KatoS1410/InterviewAssistant-Python.git
cd InterviewAssistant-Python
cargo build --release
# Binary: target/release/interview-assistant.exe (Windows)
#         target/release/interview-assistant (Linux/macOS)
```

### Architecture

```
src/
├── app.rs              # egui App, orchestration
├── config.rs           # JSON config load/save
├── main.rs             # Entry point
├── core/
│   ├── mod.rs          # Re-exports
│   ├── devices.rs      # Audio device enumeration
│   ├── helpers.rs      # Timestamp, string utils
│   ├── single_instance.rs  # Single-instance guard
│   ├── vosk_ffi.rs     # VOSK DLL FFI bindings
│   └── whisper_ffi.rs  # (legacy, unused)
├── services/
│   ├── mod.rs
│   ├── ai.rs           # AI client (OpenAI-compatible API)
│   ├── audio.rs        # Audio capture (cpal/WASAPI)
│   ├── hotkeys.rs      # Global hotkey hooks
│   └── transcriber.rs  # VOSK worker thread
└── ui/
    ├── mod.rs
    ├── theme.rs        # Dark glass theme
    ├── widgets.rs      # Reusable widgets (status bar)
    ├── main_tab.rs     # Main tab: transcript + AI answer
    ├── history_tab.rs  # History of Q&A pairs
    ├── settings_tab.rs # Configuration UI
    └── logs_tab.rs     # Application logs
```

### Requirements

- **Windows 10+** (primary target) or Linux (PulseAudio/PipeWire)
- **Rust 1.75+** (https://rustup.rs)
- **VOSK model** (~1.5 GB, downloaded separately)
- **Loopback device** (Stereo Mix or VB-Cable) for capturing system audio
- **Internet** — only for AI requests (STT works fully offline)

---

## Русский

Десктопный помощник для технических собеседований: **офлайн-распознавание речи (VOSK) + LLM (OpenAI / DeepSeek / OpenRouter / Ollama / GigaChat / свой endpoint)**.

Зажимаешь хоткей, говоришь, отпускаешь — приложение расшифровывает речь и отправляет в AI. Ответ появляется в правой панели.

### Возможности

- **Офлайн STT** — распознавание речи через VOSK, интернет не нужен
- **Несколько AI-провайдеров** — OpenAI, DeepSeek, OpenRouter, Ollama (локально), GigaChat, свой OpenAI-совместимый endpoint
- **Глобальные хоткеи** — `←` (системный звук/loopback) и `→` (микрофон), работают даже когда окно не в фокусе
- **Неблокирующий UI** — захват и распознавание в фоновых потоках, интерфейс не зависает
- **Тёмная glass-тема** — дизайн в стиле iOS. Не знаю кому как, а мне такое зашло.
- **Вкладка «История»** — все пары вопрос-ответ сохраняются для просмотра
- **Настраиваемый** — JSON-конфиг, редактируется в приложении или вручную

### Быстрый старт (Windows)

**Вариант A — Готовый .exe:** скачай из [Releases](https://github.com/KatoS1410/InterviewAssistant-Python/releases/tag/v.1.1-rust), запусти, укажи путь к VOSK модели в Настройках.

**Вариант Б — Сборка из исходников:**
1. **Установи Rust** (если ещё нет): https://rustup.rs
2. **Скачай VOSK модель**:
   - Русская: [vosk-model-ru-0.10](https://alphacephei.com/vosk/models/vosk-model-ru-0.10.zip) (~1.4 GB)
   - Английская: [vosk-model-en-us-0.22](https://alphacephei.com/vosk/models/vosk-model-en-us-0.22.zip) (~1.8 GB)
   - Другие модели: https://alphacephei.com/vosk/models
3. **Распакуй** модель в папку (например, `C:\vosk\vosk-model-ru-0.10`)
4. **Собери и запусти**:
   ```bat
   cargo build --release
   .\target\release\interview-assistant.exe
   ```
5. **Первый запуск**: вкладка «Настройки» → нажми «Обзор» рядом с путём к VOSK модели → выбери папку с моделью → нажми «Загрузить VOSK» (DLL скачается автоматически)
6. **Настрой AI**: во вкладке «Настройки» укажи API ключ, модель и endpoint
7. **Настрой аудиоустройства**: нажми «Найти Loopback» и «Найти микрофон», или выбери вручную

### Быстрый старт (Linux — Debian 12+ / Ubuntu 22.04+)

**Вариант A — Установка одной командой:**
```bash
curl -fsSL https://raw.githubusercontent.com/KatoS1410/InterviewAssistant-Python/RustConversion/linux/install.sh | bash
```
Скрипт установит системные зависимости, Rust (если нужно), склонирует репо, соберёт бинарник и создаст ярлык в меню приложений.

**Вариант Б — .deb пакет:**
```bash
# Скачай .deb из Releases, затем:
sudo dpkg -i interview-assistant_1.0.1_amd64.deb
sudo apt-get install -f   # автодоустановка недостающих зависимостей
```

**Вариант В — Ручная сборка:**
```bash
# Сначала установи системные зависимости:
sudo apt-get install -y build-essential pkg-config libasound2-dev libgtk-3-dev libx11-dev libxcb1-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libwayland-dev curl git cmake
# Затем собери:
git clone -b RustConversion https://github.com/KatoS1410/InterviewAssistant-Python.git
cd InterviewAssistant-Python
cargo build --release
sudo cp target/release/interview-assistant /usr/local/bin/
```

После установки скачай VOSK модель, распакуй и укажи путь в Настройках приложения.

### Хоткеи

| Действие | Клавиша |
|---|---|
| Запись системного звука (loopback) | `←` удерживать |
| Запись с микрофона | `→` удерживать |
| Стоп + распознавание + отправить в AI | отпустить клавишу |

### Настройка звука

**Loopback** (захват системного звука — например, голос собеседника из Zoom/Telegram):
- Windows: включи «Stereo Mix» в настройках звука, или установи [VB-Cable](https://vb-audio.com/Cable/)
- Приложение автоматически находит loopback-устройства

**Микрофон**: любой стандартный микрофон, определяется автоматически

### AI-провайдеры

| Провайдер | Endpoint | Примечания |
|---|---|---|
| OpenAI | `https://api.openai.com/v1` | Нужен API ключ |
| DeepSeek | `https://api.deepseek.com/v1` | Нужен API ключ |
| OpenRouter | `https://openrouter.ai/api/v1` | Нужен API ключ |
| Ollama | `http://localhost:11434/v1` | Бесплатно, локально, без ключа |
| GigaChat | `https://gigachat.devices.sberbank.ru/api/v1` | Нужен API ключ Сбера |
| Свой | Любой OpenAI-совместимый URL | Свой endpoint |

### Конфигурация

Файл конфига: `%APPDATA%\katos_interview_assistant\config.json` (Windows) или `~/.katos_interview_assistant/config.json` (Linux/macOS)

Основные настройки:
- `model` — название AI модели (например, `gpt-4o`, `deepseek-chat`, `llama3`)
- `api_key` — твой API ключ
- `endpoint` — URL провайдера
- `vosk_model_path` — путь к распакованной папке с VOSK моделью
- `loopback_device` / `mic_device` — названия аудиоустройств
- `chunk_ms` — размер аудио-чанка в мс (по умолчанию: 500)
- `auto_ask_sec` — автоотправка в AI через N секунд тишины (0 = выключено)

### Сборка из исходников

```bash
# Требования: Rust 1.75+, Git
git clone -b RustConversion https://github.com/KatoS1410/InterviewAssistant-Python.git
cd InterviewAssistant-Python
cargo build --release
# Бинарник: target/release/interview-assistant.exe (Windows)
#           target/release/interview-assistant (Linux/macOS)
```

### Архитектура

```
src/
├── app.rs              # egui App, оркестрация
├── config.rs           # JSON конфиг (загрузка/сохранение)
├── main.rs             # Точка входа
├── core/
│   ├── mod.rs          # Реэкспорты
│   ├── devices.rs      # Перечисление аудиоустройств
│   ├── helpers.rs      # Временные метки, утилиты строк
│   ├── single_instance.rs  # Защита от повторного запуска
│   ├── vosk_ffi.rs     # FFI-биндинги к VOSK DLL
│   └── whisper_ffi.rs  # (устаревший, не используется)
├── services/
│   ├── mod.rs
│   ├── ai.rs           # AI клиент (OpenAI-совместимый API)
│   ├── audio.rs        # Захват аудио (cpal/WASAPI)
│   ├── hotkeys.rs      # Глобальные хоткеи
│   └── transcriber.rs  # VOSK воркер (фоновый поток)
└── ui/
    ├── mod.rs
    ├── theme.rs        # Тёмная glass-тема
    ├── widgets.rs      # Переиспользуемые виджеты (статус-бар)
    ├── main_tab.rs     # Главная вкладка: транскрипт + ответ AI
    ├── history_tab.rs  # История вопросов и ответов
    ├── settings_tab.rs # UI настроек
    └── logs_tab.rs     # Логи приложения
```

### Требования

- **Windows 10+** (основная цель) или Linux (PulseAudio/PipeWire)
- **Rust 1.75+** (https://rustup.rs)
- **VOSK модель** (~1.5 GB, скачивается отдельно)
- **Loopback-устройство** (Stereo Mix или VB-Cable) для захвата системного звука
- **Интернет** — только для AI-запросов (STT работает полностью офлайн)

### Как это работает

1. Зажимаешь `←` (loopback) или `→` (микрофон) — начинается захват аудио
2. Аудио накапливается в буфер
3. Отпускаешь клавишу — захват останавливается, буфер целиком отправляется в VOSK
4. VOSK распознаёт речь (офлайн, ~1-2 секунды)
5. Распознанный текст появляется в левой панели и автоматически отправляется в AI
6. AI отвечает — ответ появляется в правой панели
7. Пара вопрос-ответ сохраняется во вкладке «История»

### Релизы

Готовые `.exe` файлы доступны в [Releases](https://github.com/KatoS1410/InterviewAssistant-Python/releases).

Для запуска готового `.exe` всё равно нужна VOSK модель — скачай и распакуй её отдельно (см. «Быстрый старт»).
