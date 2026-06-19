"""
build_windows.py — полная сборка KatoS Interview Assistant
===========================================================
Запускай: python build_windows.py
Делает всё сам. Не нужно открывать installer.iss вручную.
"""

import sys
import os
import subprocess
import shutil
from pathlib import Path

HERE  = Path(__file__).parent.resolve()
DIST  = HERE / "dist" / "KatoS_Interview_Assistant"
BUILD = HERE / "build"
SPEC  = HERE / "katos.spec"
ISS   = HERE / "installer.iss"
OUT   = HERE / "installer_output"

INNO_PATHS = [
    Path(r"C:\Program Files (x86)\Inno Setup 6\ISCC.exe"),
    Path(r"C:\Program Files\Inno Setup 6\ISCC.exe"),
    Path(r"C:\Program Files (x86)\Inno Setup 5\ISCC.exe"),
    Path(r"C:\Program Files\Inno Setup 5\ISCC.exe"),
]

REQUIRED_PACKAGES = [
    "pyinstaller", "vosk", "sounddevice", "numpy",
    "openai", "pynput", "requests", "httpx",
]

# ─────────────────────────────────────────────────────────────────────────

def run(cmd, check=True, **kw):
    print(f"  > {' '.join(str(c) for c in cmd)}")
    r = subprocess.run(cmd, **kw)
    if check and r.returncode != 0:
        print(f"\n[ОШИБКА] Код возврата: {r.returncode}")
        sys.exit(r.returncode)
    return r


def step(n, total, msg):
    print(f"\n{'─'*58}")
    print(f"  Шаг {n}/{total} — {msg}")
    print(f"{'─'*58}")


# ─────────────────────────────────────────────────────────────────────────

def ensure_packages():
    step(1, 3, "Проверяю зависимости")
    import importlib
    missing = []
    for pkg in REQUIRED_PACKAGES:
        try:
            importlib.import_module(pkg.replace("-", "_"))
        except ImportError:
            missing.append(pkg)
    if missing:
        print(f"  [..] Устанавливаю: {', '.join(missing)}")
        run([sys.executable, "-m", "pip", "install"] + missing)
    else:
        print(f"  [OK] Все {len(REQUIRED_PACKAGES)} пакетов найдены")


def build_app():
    step(2, 3, "Сборка приложения (PyInstaller)")

    # Чистим старое
    for p in [DIST.parent, BUILD, SPEC]:
        p = Path(p)
        if p.exists():
            shutil.rmtree(p) if p.is_dir() else p.unlink()

    import numpy
    numpy_dir = str(Path(numpy.__file__).parent)

    spec_text = f'''\
# -*- mode: python ; coding: utf-8 -*-
from PyInstaller.utils.hooks import collect_all

vosk_d,   vosk_b,   vosk_h   = collect_all("vosk")
sd_d,     sd_b,     sd_h     = collect_all("sounddevice")
np_d,     np_b,     np_h     = collect_all("numpy")
cffi_d,   cffi_b,   cffi_h   = collect_all("cffi")
openai_d, openai_b, openai_h = collect_all("openai")
httpx_d,  httpx_b,  httpx_h  = collect_all("httpx")
req_d,    req_b,    req_h    = collect_all("requests")

a = Analysis(
    [r"{HERE / "main_windows.py"}"],
    pathex=[r"{HERE}", r"{numpy_dir}"],
    binaries  = vosk_b + sd_b + np_b + cffi_b + openai_b + httpx_b + req_b,
    datas     = vosk_d + sd_d + np_d + cffi_d + openai_d + httpx_d + req_d,
    hiddenimports = (
        vosk_h + sd_h + np_h + cffi_h + openai_h + httpx_h + req_h + [
        "numpy", "numpy.core", "numpy.core._multiarray_umath",
        "numpy.core.multiarray", "numpy.lib", "numpy.linalg", "numpy.fft",
        "numpy.random", "numpy.polynomial",
        "pynput.keyboard._win32", "pynput.mouse._win32", "pynput._util.win32",
        "tkinter", "tkinter.ttk", "tkinter.filedialog", "tkinter.messagebox",
        "json", "threading", "pathlib", "ctypes",
    ]),
    excludes=["matplotlib", "scipy", "PIL", "cv2", "PyQt5", "wx", "test"],
    noarchive=False,
)

pyz = PYZ(a.pure)

exe = EXE(
    pyz, a.scripts, [],
    exclude_binaries=True,
    name="KatoS_Interview_Assistant",
    console=False,
    icon=None,
)

coll = COLLECT(
    exe, a.binaries, a.datas,
    strip=False, upx=True,
    name="KatoS_Interview_Assistant",
)
'''
    SPEC.write_text(spec_text, encoding="utf-8")
    print("  Сборка займёт 2-5 минут...")
    run([sys.executable, "-m", "PyInstaller", "--clean", "--noconfirm", str(SPEC)])

    exe_path = DIST / "KatoS_Interview_Assistant.exe"
    if not exe_path.exists():
        print(f"\n[ОШИБКА] .exe не найден после сборки: {exe_path}")
        print("  Проверь вывод PyInstaller выше на наличие ошибок.")
        sys.exit(1)

    size_mb = sum(f.stat().st_size for f in DIST.rglob("*") if f.is_file()) / 1024 / 1024
    print(f"\n  [OK] Собрано ({size_mb:.0f} МБ)  →  {DIST}")


def build_installer():
    step(3, 3, "Сборка установщика (Inno Setup)")

    # Проверка что dist\ реально существует перед вызовом Inno Setup
    if not DIST.exists():
        print(f"\n[ОШИБКА] Папка {DIST} не найдена.")
        print("  PyInstaller не завершил сборку. Проверь ошибки выше.")
        sys.exit(1)

    iscc = next((p for p in INNO_PATHS if p.exists()), None)
    if not iscc:
        found = shutil.which("ISCC")
        if found:
            iscc = Path(found)

    if not iscc:
        print("""
  [!] Inno Setup не найден.
      1. Скачай: https://jrsoftware.org/isdl.php
      2. Установи
      3. Снова запусти: python build_windows.py
""")
        return

    OUT.mkdir(exist_ok=True)
    print(f"  ISCC: {iscc}")
    result = run([str(iscc), str(ISS)], check=False)

    if result.returncode == 0:
        setup_exe = OUT / "KatoS_Interview_Assistant_Setup.exe"
        if setup_exe.exists():
            size_mb = setup_exe.stat().st_size / 1024 / 1024
            print(f"""
╔══════════════════════════════════════════════════════════╗
║                      ГОТОВО!                            ║
╠══════════════════════════════════════════════════════════╣
║  installer_output\\KatoS_Interview_Assistant_Setup.exe  ║
║  Размер: {size_mb:.0f} МБ                                        ║
╚══════════════════════════════════════════════════════════╝
""")
    else:
        print(f"\n[ОШИБКА] Inno Setup завершился с кодом {result.returncode}")


# ─────────────────────────────────────────────────────────────────────────

def main():
    print("=" * 58)
    print("  KatoS Interview Assistant — Windows Build")
    print("=" * 58)
    print(f"  Папка: {HERE}\n")

    ensure_packages()
    build_app()
    build_installer()


if __name__ == "__main__":
    main()
