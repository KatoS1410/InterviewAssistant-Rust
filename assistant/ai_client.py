"""
ai_client.py — OpenAI и Ollama клиенты.
"""

from __future__ import annotations

import logging
import json
from typing import Callable, List, Optional

log = logging.getLogger(__name__)

Message = dict


class ConversationHistory:
    MAX_MESSAGES = 20

    def __init__(self) -> None:
        self._system: Optional[str] = None
        self._messages: List[Message] = []

    def set_system(self, prompt: str) -> None:
        self._system = prompt

    def add_user(self, text: str) -> None:
        self._messages.append({"role": "user", "content": text})
        self._trim()

    def add_assistant(self, text: str) -> None:
        self._messages.append({"role": "assistant", "content": text})
        self._trim()

    def build(self) -> List[Message]:
        result = []
        if self._system:
            result.append({"role": "system", "content": self._system})
        result.extend(self._messages)
        return result

    def clear(self) -> None:
        self._messages.clear()

    def _trim(self) -> None:
        if len(self._messages) > self.MAX_MESSAGES:
            self._messages = self._messages[-self.MAX_MESSAGES:]


class OpenAIClient:
    def __init__(self, api_key: str, model: str, base_url: str) -> None:
        self.api_key = api_key
        self.model = model
        self.base_url = base_url.rstrip("/")

    def send(self, messages: List[Message],
             on_token: Optional[Callable[[str], None]] = None) -> str:
        from openai import OpenAI
        client = OpenAI(api_key=self.api_key, base_url=self.base_url)

        if on_token:
            full = []
            with client.chat.completions.create(
                model=self.model,
                messages=messages,
                stream=True,
                temperature=0.3,
            ) as stream:
                for chunk in stream:
                    delta = chunk.choices[0].delta.content or ""
                    if delta:
                        full.append(delta)
                        on_token(delta)
            return "".join(full)
        else:
            response = client.chat.completions.create(
                model=self.model, messages=messages, temperature=0.3,
            )
            return response.choices[0].message.content or ""

    def test_connection(self) -> tuple[bool, str]:
        try:
            self.send([{"role": "user", "content": "ping"}])
            return True, "Соединение успешно"
        except Exception as e:
            return False, str(e)


class OllamaClient:
    def __init__(self, base_url: str, model: str) -> None:
        self.base_url = base_url.rstrip("/")
        self.model = model

    def send(self, messages: List[Message],
             on_token: Optional[Callable[[str], None]] = None) -> str:
        import requests

        url = f"{self.base_url}/api/chat"
        payload = {
            "model": self.model,
            "messages": messages,
            "stream": True,
            "options": {"temperature": 0.3},
        }

        full = []
        try:
            with requests.post(url, json=payload, stream=True, timeout=180) as r:
                r.raise_for_status()
                for raw_line in r.iter_lines():
                    if not raw_line:
                        continue
                    # raw_line is bytes
                    line = raw_line.decode("utf-8", errors="replace") if isinstance(raw_line, bytes) else raw_line
                    try:
                        data = json.loads(line)
                    except json.JSONDecodeError:
                        continue

                    # Ollama /api/chat format: {"message": {"role": ..., "content": ...}, "done": bool}
                    token = data.get("message", {}).get("content", "")
                    if token:
                        full.append(token)
                        if on_token:
                            on_token(token)

                    if data.get("done", False):
                        break
        except requests.exceptions.ConnectionError:
            raise RuntimeError(
                f"Не удалось подключиться к Ollama по адресу {self.base_url}. "
                f"Убедись что Ollama запущена: ollama serve"
            )
        except requests.exceptions.HTTPError as e:
            raise RuntimeError(f"Ollama HTTP ошибка: {e}. Проверь название модели '{self.model}'.")

        return "".join(full)

    def test_connection(self) -> tuple[bool, str]:
        import requests
        try:
            r = requests.get(f"{self.base_url}/api/tags", timeout=5)
            r.raise_for_status()
            models = [m["name"] for m in r.json().get("models", [])]
            if not models:
                return (False,
                    f"Ollama доступна, но моделей нет. "
                    f"Запусти: ollama pull {self.model}")
            # Найти точное совпадение или частичное
            match = next((m for m in models if self.model in m), None)
            if match:
                return True, f"Ollama OK. Модель '{match}' найдена."
            else:
                return (False,
                    f"Модель '{self.model}' не найдена. "
                    f"Доступные: {', '.join(models[:5])}. "
                    f"Запусти: ollama pull {self.model}")
        except requests.exceptions.ConnectionError:
            return False, f"Ollama недоступна по адресу {self.base_url}. Запусти: ollama serve"
        except Exception as e:
            return False, f"Ошибка: {e}"


class AISession:
    def __init__(self) -> None:
        self.history = ConversationHistory()
        self._client = None

    def configure_openai(self, api_key: str, model: str, base_url: str) -> None:
        self._client = OpenAIClient(api_key, model, base_url)

    def configure_ollama(self, base_url: str, model: str) -> None:
        self._client = OllamaClient(base_url, model)

    def init_session(self, position: str, prompt_template: str) -> None:
        self.history.clear()
        system = prompt_template.replace("{position}", position)
        self.history.set_system(system)

    def ask(self, question: str,
            on_token: Optional[Callable[[str], None]] = None) -> str:
        if self._client is None:
            raise RuntimeError("AI клиент не настроен. Проверь настройки.")
        self.history.add_user(question)
        answer = self._client.send(self.history.build(), on_token=on_token)
        self.history.add_assistant(answer)
        return answer

    def test_connection(self) -> tuple[bool, str]:
        if self._client is None:
            return False, "Клиент не настроен"
        return self._client.test_connection()


session = AISession()
