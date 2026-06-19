"""
audio.py — захват аудио с микрофона и системного звука (loopback).

Принципы:
- Аудио копится прямо в памяти (numpy чанки).
- Нет временных файлов.
- Широкий поиск loopback — включая Кириллицу ("Стерео микшер").
"""

from __future__ import annotations

import logging
from typing import Optional

import numpy as np

log = logging.getLogger(__name__)

SAMPLE_RATE = 16000
CHANNELS = 1
DTYPE = "int16"

# Ключевые слова для поиска loopback-устройства (латиница + кириллица)
_LOOPBACK_KEYWORDS = [
    "loopback",
    "stereo mix",
    "стерео микшер",
    "стереомикшер",
    "what u hear",
    "wave out",
    "mixout",
    "mix out",
    "внутреннее аудио",
    "output mix",
    "sum",
    "cable output",    # VB-Cable
    "virtual audio",
    "blackhole",       # macOS
    "pulse",
]


def list_all_input_devices() -> list[dict]:
    """Вернуть ВСЕ устройства ввода (включая loopback)."""
    import sounddevice as sd
    devices = sd.query_devices()
    result = []
    for i, dev in enumerate(devices):
        if dev["max_input_channels"] > 0:
            result.append({
                "index": i,
                "name": dev["name"],
                "channels": dev["max_input_channels"],
            })
    return result


def find_loopback_device() -> Optional[int]:
    """
    Найти loopback устройство по ключевым словам.
    Поиск регистронезависимый, поддерживает кириллицу.
    """
    import sounddevice as sd
    devices = sd.query_devices()
    for i, dev in enumerate(devices):
        if dev["max_input_channels"] <= 0:
            continue
        name_lower = dev["name"].lower()
        for kw in _LOOPBACK_KEYWORDS:
            if kw in name_lower:
                return i
    return None


class AudioRecorder:
    """
    Запись аудио с выбранного устройства.

        recorder = AudioRecorder(device_index=None)  # None = mic по умолчанию
        recorder.start(max_seconds=20)
        audio = recorder.stop()  # numpy int16 array
    """

    def __init__(self, device_index: Optional[int] = None) -> None:
        self.device_index = device_index
        self._chunks: list[np.ndarray] = []
        self._stream = None
        self._recording = False

    @property
    def is_recording(self) -> bool:
        return self._recording

    def start(self, max_seconds: int = 20) -> None:
        import sounddevice as sd

        if self._recording:
            return

        self._chunks.clear()
        self._recording = True
        max_frames = SAMPLE_RATE * max_seconds

        def callback(indata: np.ndarray, frames: int, time, status) -> None:
            if status:
                log.debug("Audio status: %s", status)
            self._chunks.append(indata.copy())

            total = sum(len(c) for c in self._chunks)
            if total >= max_frames:
                raise sd.CallbackStop()

        try:
            self._stream = sd.InputStream(
                device=self.device_index,
                samplerate=SAMPLE_RATE,
                channels=CHANNELS,
                dtype=DTYPE,
                blocksize=1024,
                callback=callback,
            )
            self._stream.start()
        except Exception as e:
            self._recording = False
            log.error("Ошибка открытия аудио устройства: %s", e)
            raise

    def stop(self) -> np.ndarray:
        if not self._recording:
            return np.array([], dtype=np.int16)

        self._recording = False

        if self._stream is not None:
            try:
                self._stream.stop()
                self._stream.close()
            except Exception as e:
                log.debug("Stream close: %s", e)
            self._stream = None

        if not self._chunks:
            return np.array([], dtype=np.int16)

        audio = np.concatenate(self._chunks, axis=0).flatten()
        self._chunks.clear()
        return audio
