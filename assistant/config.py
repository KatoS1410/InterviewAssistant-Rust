"""
config.py — хранение настроек приложения (JSON).
"""

import json
import os
from pathlib import Path

_CONFIG_DIR = Path(os.path.expanduser("~")) / ".katos_interview_assistant"
_CONFIG_FILE = _CONFIG_DIR / "config.json"

DEFAULTS = {
    "ai_backend": "openai",

    "openai_api_key": "",
    "openai_model": "gpt-4o-mini",
    "openai_base_url": "https://api.openai.com/v1",

    "ollama_base_url": "http://localhost:11434",
    "ollama_model": "llama3",

    "vosk_model_path": "",
    "max_record_seconds": 20,
    "position": "",
    "loopback_device_index": -1,

    "system_prompt_template": (
        "Ты профессиональный {position}. Ты проходишь собеседование. "
        "Отвечай чётко, по делу, уверенно. "
        "Не упоминай, что ты ИИ."
    ),
}


def load() -> dict:
    _CONFIG_DIR.mkdir(parents=True, exist_ok=True)
    if not _CONFIG_FILE.exists():
        save(DEFAULTS.copy())
        return DEFAULTS.copy()
    try:
        with open(_CONFIG_FILE, "r", encoding="utf-8") as f:
            data = json.load(f)
        merged = DEFAULTS.copy()
        merged.update(data)
        return merged
    except (json.JSONDecodeError, OSError):
        return DEFAULTS.copy()


def save(cfg: dict) -> None:
    _CONFIG_DIR.mkdir(parents=True, exist_ok=True)
    with open(_CONFIG_FILE, "w", encoding="utf-8") as f:
        json.dump(cfg, f, ensure_ascii=False, indent=2)
