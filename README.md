# Interview Assistant

```
╔════════════════════════════════════════════════════════╗
║  Hold hotkey → Speak → Release → Get AI Answer         ║
║  Offline STT (VOSK) + Multiple LLM Providers           ║
╚════════════════════════════════════════════════════════╝
```

**[English](#english) | [Русский](#русский)**

---

## English

Desktop assistant for tech interviews. Press `←` or `→`, speak, release — speech gets transcribed offline (VOSK) and sent to AI.

### Quick Start

#### Windows

**Prebuilt:**
1. Download `.exe` from [Releases](https://github.com/KatoS1410/InterviewAssistant-Rust/releases)
2. Download VOSK model (~1.5 GB):
   - Russian: [vosk-model-ru-0.10](https://alphacephei.com/vosk/models/vosk-model-ru-0.10.zip)
   - English: [vosk-model-en-us-0.22](https://alphacephei.com/vosk/models/vosk-model-en-us-0.22.zip)
3. Extract to folder (e.g., `C:\vosk\vosk-model-ru-0.10`)
4. Run app → Settings → Browse → select model → "Load VOSK"
5. Set API key and provider in Settings

**From source:**
```bash
# Install Rust: https://rustup.rs
cargo build --release
.\target\release\interview-assistant.exe
```

#### Linux (Debian 12+, Ubuntu 22.04+)

```bash
# One-liner:
curl -fsSL https://raw.githubusercontent.com/KatoS1410/InterviewAssistant-Rust/main/linux/install.sh | bash

# Or manual:
sudo apt-get install -y build-essential pkg-config libasound2-dev libgtk-3-dev libx11-dev libxcb1-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libwayland-dev curl git cmake
git clone https://github.com/KatoS1410/InterviewAssistant-Rust.git
cd InterviewAssistant-Rust
cargo build --release
sudo cp target/release/interview-assistant /usr/local/bin/
```

### Hotkeys

| Action | Key |
|--------|-----|
| Record system audio (loopback) | `←` hold |
| Record microphone | `→` hold |
| Stop + send to AI | Release |

### Audio Setup

**Loopback** (system audio):
- Windows: Enable "Stereo Mix" in Sound settings or install [VB-Cable](https://vb-audio.com/Cable/)
- Linux: PulseAudio/PipeWire auto-detected

**Microphone**: Any standard mic, auto-detected

### AI Providers

| Provider | Endpoint | Notes |
|----------|----------|-------|
| OpenAI | `https://api.openai.com/v1` | API key required |
| DeepSeek | `https://api.deepseek.com/v1` | Cheaper, fast |
| OpenRouter | `https://openrouter.ai/api/v1` | Multi-model |
| Ollama | `http://localhost:11434/v1` | Free, local, no key |
| GigaChat | `https://gigachat.devices.sberbank.ru/api/v1` | Russian |
| Custom | Any OpenAI-compatible | Your endpoint |

### Config

**Location:**
- Windows: `%APPDATA%\katos_interview_assistant\config.json`
- Linux/macOS: `~/.katos_interview_assistant/config.json`

**Key settings:**
- `model` — AI model (e.g., `gpt-4o`, `deepseek-chat`)
- `api_key` — Your API key
- `endpoint` — Provider URL
- `vosk_model_path` — Path to VOSK model folder
- `chunk_ms` — Audio buffer size (default: 500)
- `tail_ms` — Loopback tail delay (default: 6000, increase if audio cuts off)

### Requirements

- Windows 10+ or Linux (PulseAudio/PipeWire)
- Rust 1.75+
- VOSK model (~1.5 GB, separate download)
- Loopback device for system audio (optional)
- Internet only for AI (STT is fully offline, but you can use Ollama as well)

---

## Русский

Десктопный помощник для собеседований. Нажми `←` или `→`, говори, отпусти — речь распознаётся офлайн и отправляется AI.

### Быстрый старт

#### Windows

**Готовый бинарник:**
1. Скачай `.exe` из [Releases](https://github.com/KatoS1410/InterviewAssistant-Rust/releases)
2. Скачай VOSK модель (~1.5 GB):
   - Русская: [vosk-model-ru-0.10](https://alphacephei.com/vosk/models/vosk-model-ru-0.10.zip)
   - Английская: [vosk-model-en-us-0.22](https://alphacephei.com/vosk/models/vosk-model-en-us-0.22.zip)
3. Распакуй в папку (например, `C:\vosk\vosk-model-ru-0.10`)
4. Запусти приложение → Настройки → Обзор → выбери модель → "Загрузить VOSK"
5. Укажи API ключ и провайдера в Настройках

**Из исходников:**
```bash
# Установи Rust: https://rustup.rs
cargo build --release
.\target\release\interview-assistant.exe
```

#### Linux (Debian 12+, Ubuntu 22.04+)

```bash
# Одна команда:
curl -fsSL https://raw.githubusercontent.com/KatoS1410/InterviewAssistant-Rust/main/linux/install.sh | bash

# Или вручную:
sudo apt-get install -y build-essential pkg-config libasound2-dev libgtk-3-dev libx11-dev libxcb1-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libwayland-dev curl git cmake
git clone https://github.com/KatoS1410/InterviewAssistant-Rust.git
cd InterviewAssistant-Rust
cargo build --release
sudo cp target/release/interview-assistant /usr/local/bin/
```

### Хоткеи

| Действие | Клавиша |
|----------|---------|
| Запись системного звука | `←` удерживать |
| Запись микрофона | `→` удерживать |
| Стоп + отправить в AI | Отпустить |

### Звук

**Loopback** (системный звук):
- Windows: Включи "Stereo Mix" или установи [VB-Cable](https://vb-audio.com/Cable/)
- Linux: PulseAudio/PipeWire определяется автоматически

**Микрофон**: Любой стандартный, определяется автоматически

### AI провайдеры

| Провайдер | Endpoint | Примечания |
|-----------|----------|-----------|
| OpenAI | `https://api.openai.com/v1` | API ключ |
| DeepSeek | `https://api.deepseek.com/v1` | Дешевле |
| OpenRouter | `https://openrouter.ai/api/v1` | Много моделей |
| Ollama | `http://localhost:11434/v1` | Бесплатно, локально |
| GigaChat | `https://gigachat.devices.sberbank.ru/api/v1` | Русский |
| Свой | Любой OpenAI-совместимый | Свой сервер |

### Конфиг

**Расположение:**
- Windows: `%APPDATA%\katos_interview_assistant\config.json`
- Linux/macOS: `~/.katos_interview_assistant/config.json`

**Основные параметры:**
- `model` — модель AI (например, `gpt-4o`, `deepseek-chat`)
- `api_key` — Твой API ключ
- `endpoint` — URL провайдера
- `vosk_model_path` — Путь к папке с VOSK
- `chunk_ms` — Размер буфера аудио (по умолчанию: 500)
- `tail_ms` — Задержка сбора хвоста loopback (по умолчанию: 6000, увеличь если обрезается конец)

### Требования

- Windows 10+ или Linux (PulseAudio/PipeWire)
- Rust 1.75+
- VOSK модель (~1.5 GB, скачивается отдельно)
- Loopback-устройство для системного звука (опционально)
- Интернет только для AI (STT работает полностью офлайн, но для полного оффлайна можете использовать Ollama)
