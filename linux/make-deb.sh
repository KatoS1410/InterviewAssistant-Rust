#!/bin/bash
# Собирает .deb установщик на Linux.
# Использование: ./linux/make-deb.sh
# На выходе: interview-assistant_1.0.1_amd64.deb
set -euo pipefail

GREEN='\033[0;32m'; NC='\033[0m'
log() { echo -e "${GREEN}[+]${NC} $*"; }

PKG_NAME="interview-assistant"
PKG_VER="1.0.1"
PKG_ARCH="amd64"
DEB_NAME="${PKG_NAME}_${PKG_VER}_${PKG_ARCH}.deb"
BUILD_DIR="/tmp/${PKG_NAME}-deb"

# Чистим и создаём сборочную папку.
log "Создаю структуру пакета..."
rm -rf "$BUILD_DIR"
mkdir -p "${BUILD_DIR}/DEBIAN"
mkdir -p "${BUILD_DIR}/usr/local/bin"
mkdir -p "${BUILD_DIR}/usr/share/applications"
mkdir -p "${BUILD_DIR}/usr/share/${PKG_NAME}"

# Копируем установочный скрипт в пакет.
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cp "${SCRIPT_DIR}/install.sh" "${BUILD_DIR}/usr/share/${PKG_NAME}/install.sh"
chmod 755 "${BUILD_DIR}/usr/share/${PKG_NAME}/install.sh"

# Метаданные пакета (control).
cat > "${BUILD_DIR}/DEBIAN/control" <<EOF
Package: ${PKG_NAME}
Version: ${PKG_VER}
Section: utils
Priority: optional
Architecture: ${PKG_ARCH}
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
EOF

# Пост-установочный скрипт (postinst).
cat > "${BUILD_DIR}/DEBIAN/postinst" <<'POSTINST'
#!/bin/bash
set -euo pipefail

GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'
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
POSTINST
chmod 755 "${BUILD_DIR}/DEBIAN/postinst"

# Собираем .deb.
log "Собираю ${DEB_NAME}..."
dpkg-deb --build "$BUILD_DIR" "$DEB_NAME"

# Готово.
log "Пакет создан: $(pwd)/${DEB_NAME}"
echo ""
echo "Установка на целевой машине:"
echo "  sudo dpkg -i ${DEB_NAME}"
echo ""
echo "Потом скачай VOSK-модель и настрой в приложении."
