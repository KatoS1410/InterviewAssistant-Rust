#!/usr/bin/env python3
"""Собирает .deb пакет на любой платформе (dpkg-deb не нужен).
Создаёт ar-архив из debian-binary + control.tar.gz + data.tar.gz."""
import os, sys, tarfile, io, struct, tempfile, shutil

PKG_NAME = "interview-assistant"
PKG_VER = "1.0.1"
PKG_ARCH = "amd64"
DEB_NAME = f"{PKG_NAME}_{PKG_VER}_{PKG_ARCH}.deb"
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))

def make_tar_gz(paths: dict[str, str]) -> bytes:
    """Создаёт .tar.gz из словаря {arcname: путь_на_диске}."""
    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode="w:gz") as tar:
        for arcname, fpath in paths.items():
            tar.add(fpath, arcname=arcname)
    return buf.getvalue()

def ar_pad(data: bytes) -> bytes:
    """Добивает до чётной длины (требование формата ar)."""
    if len(data) % 2 == 1:
        return data + b"\n"
    return data

def make_ar(members: list[tuple[str, bytes]]) -> bytes:
    """Создаёт ar-архив. Каждый элемент: (имя_файла, данные)."""
    buf = io.BytesIO()
    buf.write(b"!<arch>\n")
    for fname, data in members:
        # Заголовок ar: 16 байт имя, 12 — mtime, 6 — uid, 6 — gid,
        # 8 — права, 10 — размер, 2 — магическая сигнатура.
        size = len(data)
        header = f"{fname:<16}{0:<12}{0:<6}{0:<6}{100644:<8}{size:<10}`\n".encode("ascii")
        assert len(header) == 60, f"header len {len(header)}"
        buf.write(header)
        buf.write(ar_pad(data))
    return buf.getvalue()

def main():
    # Строим структуру папок пакета.
    build = tempfile.mkdtemp(prefix="deb-")
    try:
        # Файл DEBIAN/control (метаданные пакета).
        debian_dir = os.path.join(build, "DEBIAN")
        os.makedirs(debian_dir, exist_ok=True)
        control_path = os.path.join(debian_dir, "control")
        with open(control_path, "w", encoding="utf-8") as f:
            f.write(f"""Package: {PKG_NAME}
Version: {PKG_VER}
Section: utils
Priority: optional
Architecture: {PKG_ARCH}
Depends: build-essential, pkg-config, libasound2-dev, libgtk-3-dev, libx11-dev, libxcb1-dev, libxcb-shape0-dev, libxcb-xfixes0-dev, libxkbcommon-dev, libwayland-dev, curl, git, cmake
Maintainer: KatoS <kato@example.com>
Description: Offline speech + AI assistant for technical interviews
 Interview Assistant captures system audio or microphone,
 transcribes speech offline via VOSK, and sends the text
 to an LLM (OpenAI / DeepSeek / OpenRouter / Ollama / GigaChat)
 for real-time interview assistance.
 .
 This package installs system dependencies, Rust toolchain,
 clones the source, builds the binary, and creates a desktop entry.
 .
 Features:
  - Offline STT (VOSK)
  - Multiple AI providers
  - Global hotkeys (arrow keys)
  - Dark glass theme (egui)
  - History of Q&A pairs
""")

        # Файл DEBIAN/postinst (пост-установочный скрипт).
        postinst_path = os.path.join(debian_dir, "postinst")
        install_sh = os.path.join(SCRIPT_DIR, "install.sh")
        # postinst просто вызывает вложенный install.sh.
        with open(postinst_path, "w", encoding="utf-8") as f:
            f.write("""#!/bin/bash
set -euo pipefail
GREEN='\\033[0;32m'; YELLOW='\\033[1;33m'; NC='\\033[0m'
log()  { echo -e "${GREEN}[+]${NC} $*"; }
warn() { echo -e "${YELLOW}[!]${NC} $*"; }

REPO_URL="https://github.com/KatoS1410/InterviewAssistant-Python.git"
BRANCH="RustConversion"
INSTALL_DIR="/opt/interview-assistant"
BIN_NAME="interview-assistant"
DESKTOP_FILE="/usr/share/applications/interview-assistant.desktop"

log "Настройка после установки Interview Assistant..."

# Ставим Rust.
if command -v rustc &>/dev/null; then
    log "Rust уже установлен: $(rustc --version)"
else
    log "Устанавливаю Rust через rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
    source "$HOME/.cargo/env"
fi

export PATH="$HOME/.cargo/bin:$PATH"
rustup default stable 2>/dev/null || true

# Клонируем / обновляем репозиторий.
if [ -d "$INSTALL_DIR/.git" ]; then
    log "Обновляю существующий репозиторий..."
    cd "$INSTALL_DIR"
    git fetch origin "$BRANCH"
    git checkout "$BRANCH"
    git reset --hard "origin/$BRANCH"
else
    log "Клонирую репозиторий в $INSTALL_DIR..."
    mkdir -p "$INSTALL_DIR"
    git clone --branch "$BRANCH" --depth 1 "$REPO_URL" "$INSTALL_DIR"
fi

# Собираем.
cd "$INSTALL_DIR"
log "Собираю релизный бинарник (может занять пару минут)..."
cargo build --release 2>&1 | tail -5

BIN_PATH="$INSTALL_DIR/target/release/$BIN_NAME"
if [ ! -f "$BIN_PATH" ]; then
    echo "ОШИБКА: Сборка провалилась — бинарник не найден: $BIN_PATH"
    exit 1
fi

# Устанавливаем бинарник.
log "Устанавливаю бинарник в /usr/local/bin/$BIN_NAME..."
cp "$BIN_PATH" "/usr/local/bin/$BIN_NAME"
chmod +x "/usr/local/bin/$BIN_NAME"

# Ярлык рабочего стола.
log "Создаю ярлык рабочего стола..."
cat > "$DESKTOP_FILE" <<EOF
[Desktop Entry]
Name=Interview Assistant
Comment=Offline speech + AI assistant for technical interviews
Exec=/usr/local/bin/$BIN_NAME
Icon=utilities-terminal
Terminal=false
Type=Application
Categories=Utility;Office;
StartupWMClass=interview-assistant
EOF
update-desktop-database 2>/dev/null || true

# Готово.
echo ""
log "══════════════════════════════════════════════════════"
log " Interview Assistant успешно установлен!"
log "══════════════════════════════════════════════════════"
echo ""
warn "ВАЖНО: Для распознавания речи нужна VOSK-модель."
echo "  Скачать: https://alphacephei.com/vosk/models"
echo "  Распаковать и указать путь во вкладке Настройки."
echo ""
log "Запуск: interview-assistant  (или найди в меню приложений)"
echo ""
""")
        os.chmod(postinst_path, 0o755)

        # data.tar.gz: usr/share/interview-assistant/install.sh
        usr_share = os.path.join(build, "usr", "share", PKG_NAME)
        os.makedirs(usr_share, exist_ok=True)
        shutil.copy2(install_sh, os.path.join(usr_share, "install.sh"))

        # Собираем tar.gz-члены.
        # control.tar.gz
        control_members = {}
        for f in ["control", "postinst"]:
            control_members[f"./{f}"] = os.path.join(debian_dir, f)
        control_tgz = make_tar_gz(control_members)

        # data.tar.gz
        data_members = {}
        for root, dirs, files in os.walk(build):
            for fname in files:
                fpath = os.path.join(root, fname)
                arcname = "./" + os.path.relpath(fpath, build).replace("\\", "/")
                if arcname.startswith("./DEBIAN"):
                    continue
                data_members[arcname] = fpath
        data_tgz = make_tar_gz(data_members)

        # Собираем .deb (ar-архив).
        deb_content = make_ar([
            ("debian-binary", b"2.0\n"),
            ("control.tar.gz", control_tgz),
            ("data.tar.gz", data_tgz),
        ])

        out_path = os.path.join(SCRIPT_DIR, DEB_NAME)
        with open(out_path, "wb") as f:
            f.write(deb_content)

        print(f"[+] Создан: {out_path} ({len(deb_content)} байт)")
        print(f"    Установка: sudo dpkg -i {DEB_NAME}")
        print(f"    Потом скачай VOSK-модель и настрой в приложении.")
    finally:
        shutil.rmtree(build, ignore_errors=True)

if __name__ == "__main__":
    main()
