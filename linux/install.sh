#!/bin/bash
# Установщик Interview Assistant для Linux (Debian 12+ / Ubuntu 22.04+).
# Использование: chmod +x install.sh && ./install.sh
set -euo pipefail

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'
log()  { echo -e "${GREEN}[+]${NC} $*"; }
warn() { echo -e "${YELLOW}[!]${NC} $*"; }
err()  { echo -e "${RED}[x]${NC} $*"; exit 1; }

REPO_URL="https://github.com/KatoS1410/InterviewAssistant-Python.git"
BRANCH="RustConversion"
INSTALL_DIR="/opt/interview-assistant"
BIN_NAME="interview-assistant"
DESKTOP_FILE="/usr/share/applications/interview-assistant.desktop"

# Проверяем дистрибутив.
if [ -f /etc/os-release ]; then
    . /etc/os-release
    log "Обнаружен: $NAME $VERSION_ID"
    case "$ID" in
        debian) [ "${VERSION_ID%.*}" -ge 12 ] || warn "Debian <12 — некоторых пакетов может не быть";;
        ubuntu) [ "${VERSION_ID%.*}" -ge 22 ] || warn "Ubuntu <22.04 — некоторых пакетов может не быть";;
        *)      warn "Непроверенный дистрибутив: $ID. Продолжаем.";;
    esac
else
    warn "Не могу определить дистрибутив. Продолжаем."
fi

# Ставим системные зависимости.
log "Устанавливаю системные зависимости..."
sudo apt-get update -qq
sudo apt-get install -y -qq \
    build-essential \
    pkg-config \
    libasound2-dev \
    libgtk-3-dev \
    libx11-dev \
    libxcb1-dev \
    libxcb-shape0-dev \
    libxcb-xfixes0-dev \
    libxkbcommon-dev \
    libwayland-dev \
    curl \
    git \
    cmake

# Ставим Rust.
if command -v rustc &>/dev/null; then
    RUST_VER=$(rustc --version | cut -d' ' -f2)
    log "Rust уже установлен: $RUST_VER"
else
    log "Устанавливаю Rust через rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
    # shellcheck source=/dev/null
    source "$HOME/.cargo/env"
fi

# Добавляем ~/.cargo/bin в PATH для текущей сессии.
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
    sudo mkdir -p "$INSTALL_DIR"
    sudo chown "$USER:$USER" "$INSTALL_DIR"
    git clone --branch "$BRANCH" --depth 1 "$REPO_URL" "$INSTALL_DIR"
fi

# Собираем.
cd "$INSTALL_DIR"
log "Собираю релизный бинарник (может занять пару минут)..."
cargo build --release 2>&1 | tail -5

BIN_PATH="$INSTALL_DIR/target/release/$BIN_NAME"
if [ ! -f "$BIN_PATH" ]; then
    err "Сборка провалилась — бинарник не найден: $BIN_PATH"
fi

# Устанавливаем бинарник.
log "Устанавливаю бинарник в /usr/local/bin/$BIN_NAME..."
sudo cp "$BIN_PATH" "/usr/local/bin/$BIN_NAME"
sudo chmod +x "/usr/local/bin/$BIN_NAME"

# Ярлык рабочего стола.
log "Создаю ярлык рабочего стола..."
sudo tee "$DESKTOP_FILE" > /dev/null <<EOF
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
sudo update-desktop-database 2>/dev/null || true

# Напоминание про VOSK-модель.
echo ""
log "══════════════════════════════════════════════════════"
log " Установка завершена!"
log "══════════════════════════════════════════════════════"
echo ""
warn "ВАЖНО: Для распознавания речи нужна VOSK-модель."
echo "  Скачай одну из:"
echo "    • Русская:  https://alphacephei.com/vosk/models/vosk-model-ru-0.10.zip  (~1.4 GB)"
echo "    • Английская:  https://alphacephei.com/vosk/models/vosk-model-en-us-0.22.zip (~1.8 GB)"
echo "    • Ещё:     https://alphacephei.com/vosk/models"
echo ""
echo "  Распакуй куда угодно (например ~/vosk-model-ru-0.10) и укажи путь"
echo "  во вкладке Настройки при первом запуске."
echo ""
log "Запуск: interview-assistant  (или найди в меню приложений)"
echo ""
