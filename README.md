# Interview Assistant (Rust + egui)

Переписанный помощник для технических собеседований: **whisper.cpp (офлайн STT) + LLM (OpenAI / DeepSeek / OpenRouter / Ollama / GigaChat / Local LLMs)**.

Минимальный glass UI, глобальные хоткеи, автоскачивание whisper.cpp и моделей.

## Быстрый старт

### Windows
```bat
cargo build --release
.\target\release\interview-assistant.exe
```

### Linux / macOS
```bash
cargo build --release
./target/release/interview-assistant
```

При первом запуске:
1. Автоматически скачается whisper.cpp бинарник
2. Автоматически скачается модель whisper (base ~150 MB)
3. Запустится GUI

## Управление

| Действие | Клавиша |
|---|---|
| Запись системного звука (loopback) | `←` удерживать |
| Запись микрофона | `→` удерживать |
| Стоп + распознавание + AI | отпустить клавишу |

## Настройки

Файл: `~/.katos_interview_assistant/config.json`

Провайдеры: OpenAI, DeepSeek, OpenRouter, Local LLMs (Ollama), GigaChat, свой endpoint.

## Сборка release

```bash
cargo build --release
```

Бинарник: `target/release/interview-assistant` (или `.exe` на Windows)

## Архитектура

```
src/
├── app.rs           # egui App, оркестрация
├── config.rs        # JSON конфиг
├── core/            # устройства, single-instance, whisper setup
├── services/        # audio (cpal), whisper worker, AI, hotkeys
└── ui/              # glass theme + вкладки
```

## Что реализовано

- whisper.cpp в отдельном процессе — модель не блокирует GUI
- Partial-транскрипт обновляет строку, а не плодит дубликаты
- Единый worker для STT без гонок потоков
- Автоскачивание whisper.cpp и моделей (Base/Small/Medium)
- Release-сборка с LTO — быстрее и меньше
- Чистое разделение UI / services / core
- Глобальные хоткеи (стрелки влево/вправо)

## Требования

- Rust 1.75+ (https://rustup.rs)
- Windows 10+ или Linux (PulseAudio/PipeWire)
- Для loopback на Windows: Stereo Mix или VB-Cable
- Интернет для первоначальной загрузки whisper.cpp и модели
