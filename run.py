"""
KatoS Interview Assistant
=========================
Запусти: python run.py
Всё установится автоматически.
"""

import sys
import os
import subprocess

REQUIRED = [
    ("vosk",        "vosk"),
    ("sounddevice", "sounddevice"),
    ("numpy",       "numpy"),
    ("openai",      "openai"),
    ("pynput",      "pynput"),
    ("requests",    "requests"),
]


def _check_missing():
    import importlib
    missing = []
    for mod, pkg in REQUIRED:
        try:
            importlib.import_module(mod)
        except ImportError:
            missing.append(pkg)
    return missing


def _install(packages):
    print(f"[Bootstrap] Устанавливаю: {', '.join(packages)}")
    subprocess.check_call(
        [sys.executable, "-m", "pip", "install", "--quiet"] + packages
    )


def _hide_console():
    """Скрыть консольное окно на Windows."""
    if sys.platform == "win32":
        try:
            import ctypes
            hwnd = ctypes.windll.kernel32.GetConsoleWindow()
            if hwnd:
                ctypes.windll.user32.ShowWindow(hwnd, 0)  # SW_HIDE
        except Exception:
            pass


def main():
    print("=" * 50)
    print("   KatoS Interview Assistant")
    print("=" * 50)

    missing = _check_missing()
    if missing:
        print(f"[Bootstrap] Недостающие пакеты: {missing}")
        _install(missing)
        print("[Bootstrap] Перезапускаю...")
        os.execv(sys.executable, [sys.executable] + sys.argv)
        return

    print("[Bootstrap] Все зависимости на месте. Запускаю интерфейс...")
    _hide_console()

    from assistant.app import launch
    launch()


if __name__ == "__main__":
    main()
