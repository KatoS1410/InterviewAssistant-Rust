"""
KatoS Interview Assistant — Linux launcher
==========================================
Запуск: python3 run_linux.py

Автоматически создаёт venv и устанавливает зависимости.
Не трогает системный Python.
"""

import sys
import os
import subprocess
from pathlib import Path

VENV_DIR = Path(__file__).parent / ".venv"

REQUIRED = [
    ("vosk",        "vosk"),
    ("sounddevice", "sounddevice"),
    ("numpy",       "numpy"),
    ("openai",      "openai"),
    ("pynput",      "pynput"),
    ("requests",    "requests"),
]


def _is_inside_venv() -> bool:
    return (
        hasattr(sys, "real_prefix")
        or (hasattr(sys, "base_prefix") and sys.base_prefix != sys.prefix)
    )


def _venv_python() -> Path:
    return VENV_DIR / "bin" / "python"


def _create_venv():
    print(f"[Bootstrap] Создаю виртуальное окружение в {VENV_DIR} ...")
    # Убедимся, что python3-venv установлен
    result = subprocess.run(
        [sys.executable, "-m", "venv", str(VENV_DIR)],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        print("[Bootstrap] Ошибка создания venv. Попробуй:")
        print("    sudo apt install python3-venv python3-full")
        print(result.stderr)
        sys.exit(1)
    print("[Bootstrap] Окружение создано.")


def _check_missing_in_venv() -> list[str]:
    """Проверить наличие пакетов внутри venv."""
    py = str(_venv_python())
    missing = []
    for mod, pkg in REQUIRED:
        r = subprocess.run(
            [py, "-c", f"import {mod}"],
            capture_output=True,
        )
        if r.returncode != 0:
            missing.append(pkg)
    return missing


def _install_in_venv(packages: list[str]):
    py = str(_venv_python())
    print(f"[Bootstrap] Устанавливаю в venv: {', '.join(packages)}")
    r = subprocess.run(
        [py, "-m", "pip", "install", "--quiet", "--upgrade"] + packages,
    )
    if r.returncode != 0:
        print("[Bootstrap] Ошибка установки пакетов!")
        sys.exit(1)
    print("[Bootstrap] Установка завершена.")


def _check_system_deps():
    """
    Проверить наличие системных библиотек для sounddevice/portaudio.
    Выводит подсказку если чего-то не хватает.
    """
    missing_sys = []

    # portaudio нужен для sounddevice
    r = subprocess.run(
        ["dpkg", "-l", "libportaudio2"],
        capture_output=True, text=True,
    )
    if "ii" not in r.stdout:
        missing_sys.append("libportaudio2")

    # tk нужен для tkinter
    r2 = subprocess.run(
        [sys.executable, "-c", "import tkinter"],
        capture_output=True,
    )
    if r2.returncode != 0:
        missing_sys.append("python3-tk")

    if missing_sys:
        print("\n[Bootstrap] Нужны системные пакеты:")
        print(f"    sudo apt install {' '.join(missing_sys)}\n")
        sys.exit(1)


def main():
    print("=" * 50)
    print("   KatoS Interview Assistant  [Linux]")
    print("=" * 50)

    # Если уже внутри нашего venv — просто запускаем приложение
    if _is_inside_venv():
        print("[Bootstrap] Виртуальное окружение активно. Запускаю...")
        _hide_console_stub()  # на Linux нет консольного окна — ничего не делаем
        from assistant.app import launch
        launch()
        return

    # Проверка системных зависимостей
    _check_system_deps()

    # Создать venv если его нет
    if not _venv_python().exists():
        _create_venv()

    # Установить недостающие пакеты
    missing = _check_missing_in_venv()
    if missing:
        _install_in_venv(missing)

    # Перезапустить себя через venv-Python
    venv_py = str(_venv_python())
    script   = str(Path(__file__).resolve())
    print(f"[Bootstrap] Перезапускаю через venv: {venv_py}")
    os.execv(venv_py, [venv_py, script] + sys.argv[1:])


def _hide_console_stub():
    pass  # На Linux нет консольного окна — функция-заглушка


if __name__ == "__main__":
    main()
