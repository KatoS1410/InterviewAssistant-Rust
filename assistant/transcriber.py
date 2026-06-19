"""
transcriber.py — VOSK speech-to-text.

Оптимизации при загрузке тяжёлой модели (1.8 ГБ):
- Загрузка в фоновом потоке (UI не блокируется).
- Во время загрузки понижается приоритет процесса (Windows/Linux),
  чтобы система не вешалась.
- После загрузки приоритет восстанавливается, вызывается gc.collect().
- Модель держится в памяти весь сеанс — НЕ перезагружается между записями.
- Между записями вызывается только Reset() — мгновенно.
"""

from __future__ import annotations

import gc
import json
import logging
import os
import sys
import threading
from typing import Callable, Optional

log = logging.getLogger(__name__)


# ─────────────────────────── Приоритет процесса ────────────────────────────

def _lower_process_priority():
    """Понизить приоритет текущего процесса на время загрузки модели."""
    try:
        if sys.platform == "win32":
            import ctypes
            # BELOW_NORMAL_PRIORITY_CLASS = 0x4000
            ctypes.windll.kernel32.SetPriorityClass(
                ctypes.windll.kernel32.GetCurrentProcess(), 0x4000
            )
        else:
            os.nice(10)   # Linux/macOS: повышаем nice (= ниже приоритет CPU)
    except Exception as e:
        log.debug("Не удалось понизить приоритет: %s", e)


def _restore_process_priority():
    """Вернуть нормальный приоритет после загрузки."""
    try:
        if sys.platform == "win32":
            import ctypes
            # NORMAL_PRIORITY_CLASS = 0x0020
            ctypes.windll.kernel32.SetPriorityClass(
                ctypes.windll.kernel32.GetCurrentProcess(), 0x0020
            )
        else:
            os.nice(-10)  # Linux: возвращаем (требует CAP_SYS_NICE, игнорируем ошибку)
    except Exception as e:
        log.debug("Не удалось восстановить приоритет: %s", e)


# ─────────────────────────── Transcriber ───────────────────────────────────

class Transcriber:
    """Singleton-обёртка над VOSK. Одна модель — весь сеанс."""

    def __init__(self) -> None:
        self._model      = None          # vosk.Model — держим в памяти
        self._recognizer = None          # vosk.KaldiRecognizer
        self._lock       = threading.Lock()
        self._sample_rate: int = 16000
        self.loaded: bool = False
        self.model_path: str = ""

    # ───── Public API ──────────────────────────────────────────────────────

    def load(self, model_path: str, on_progress: Optional[Callable[[str], None]] = None) -> None:
        """
        Загрузить VOSK-модель.
        Вызывать из фонового потока — блокирует до завершения загрузки.
        """
        import vosk

        if self.loaded and self.model_path == model_path:
            return  # уже загружена та же модель

        self._notify(on_progress, "Загрузка модели VOSK...")

        with self._lock:
            # Освободить старую модель из памяти
            self._recognizer = None
            self._model = None
            self.loaded = False
            gc.collect()

            try:
                vosk.SetLogLevel(-1)  # Заглушить лишние логи

                self._notify(on_progress, "Читаю модель с диска (может занять ~30 сек)...")

                # Понижаем приоритет чтобы не вешать систему
                _lower_process_priority()
                try:
                    model = vosk.Model(model_path)
                finally:
                    _restore_process_priority()

                self._notify(on_progress, "Инициализирую распознаватель...")

                rec = vosk.KaldiRecognizer(model, float(self._sample_rate))
                rec.SetMaxAlternatives(0)
                rec.SetWords(False)  # без тайм-кодов — быстрее

                self._model      = model
                self._recognizer = rec
                self.model_path  = model_path
                self.loaded      = True

                # Освободить мусор после загрузки
                gc.collect()

                self._notify(on_progress, "Модель загружена ✓")

            except Exception as e:
                _restore_process_priority()
                self.loaded = False
                log.error("Ошибка загрузки модели: %s", e)
                raise

    def transcribe(self, audio_int16: "np.ndarray") -> str:
        """
        Распознать речь из numpy int16 массива (16 kHz, mono).
        Возвращает строку текста.
        """
        if not self.loaded or self._recognizer is None:
            return ""

        audio_bytes = audio_int16.tobytes()

        with self._lock:
            # Подаём аудио кусками — recognizer работает оптимальнее
            chunk = 8192
            for i in range(0, len(audio_bytes), chunk):
                self._recognizer.AcceptWaveform(audio_bytes[i:i + chunk])

            result_json = self._recognizer.FinalResult()
            self._recognizer.Reset()   # сброс состояния, НЕ перезагрузка

        try:
            return json.loads(result_json).get("text", "").strip()
        except (json.JSONDecodeError, KeyError):
            return ""

    def unload(self) -> None:
        """Освободить память модели (при закрытии приложения)."""
        with self._lock:
            self._recognizer = None
            self._model      = None
            self.loaded      = False
            self.model_path  = ""
        gc.collect()

    # ───── Internal ────────────────────────────────────────────────────────

    @staticmethod
    def _notify(cb: Optional[Callable[[str], None]], msg: str) -> None:
        if cb:
            try:
                cb(msg)
            except Exception:
                pass


# Глобальный singleton
transcriber = Transcriber()
