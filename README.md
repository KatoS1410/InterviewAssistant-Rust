# KatoS Interview Assistant
<img width="1020" height="752" alt="image" src="https://github.com/user-attachments/assets/1a3bd418-ced6-47fa-8ef9-f54c3b1ab1e2" />


Voice‑assisted interview tool. Captures system audio or microphone, transcribes with VOSK, sends to OpenAI/Ollama, displays response.



## Features

- Two input sources: loopback (desktop audio) and microphone.
- VOSK speech‑to‑text (model loaded once, kept in memory).
- AI backends: OpenAI (compatible APIs) and Ollama.
- Global hotkeys (← / →) – press and hold to record, release to transcribe + get answer.
- Conversation history (last 20 messages).
- Custom system prompt with `{position}` placeholder.

## Requirements

- Windows 10/11.
- VOSK model (download from https://alphacephei.com/vosk/models).
- For OpenAI: API key.
- For Ollama: running server (`ollama serve`) and a pulled model.

## Installation

### Pre‑built installer
Download `KatoS_Interview_Assistant_Setup.exe` from Releases. Run and follow the wizard.

### Build from source
1. Install Python 3.9+, Inno Setup 6.
2. Clone this repository.
3. Run `python build_windows.py`.  
   It installs dependencies, builds the `.exe` with PyInstaller, and compiles the installer into `installer_output/`.

## Usage

1. Launch the app.
2. Go to **Settings**: set VOSK model path, choose AI backend, configure audio devices.
3. On **Main** tab: enter your target position (e.g., "Backend Engineer").
4. Click **"Загрузить модель"** – loads VOSK and initialises the AI session.
5. Hold **←** (left arrow) to record system audio. Hold **→** (right arrow) to record from microphone.  
   Release to stop – transcription and AI reply appear automatically.

You can also edit the transcript and click **"Отправить вручную"** to send custom text to the AI.

## Configuration

Settings are stored in `%USERPROFILE%\.katos_interview_assistant\config.json`.

## Troubleshooting

- **No loopback device**: enable Stereo Mix in Windows Sound settings (Recording tab) or install a virtual audio cable.
- **VOSK model fails**: path must point to the extracted folder (contains `am`, `conf`, `graph`). Loading takes ~30 sec for large models.
- **OpenAI error**: verify API key and base URL.
- **Ollama error**: ensure `ollama serve` is running and the model name is correct (`ollama list`).
- **Recording doesn’t start**: check microphone permissions in Windows; for loopback, ensure the selected device is active and not muted.
- **My settings doesn't save after I quit the programm**: I know, I'm currently working on the patch. Just don't quit the program once you're fully loaded and done with your stuff.
## License

MIT
