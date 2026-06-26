#!/bin/bash
# Собирает .deb пакет для Interview Assistant.
# Запускать на Debian 12 / Ubuntu 22.04+ после install.sh.
# Использование: chmod +x build-deb.sh && ./build-deb.sh
set -euo pipefail

GREEN='\033[0;32m'; NC='\033[0m'
log() { echo -e "${GREEN}[+]${NC} $*"; }

PKG_NAME="interview-assistant"
PKG_VER="1.0.1"
PKG_ARCH="amd64"
DEB_NAME="${PKG_NAME}_${PKG_VER}_${PKG_ARCH}.deb"
BUILD_DIR="/tmp/${PKG_NAME}-deb"
INSTALL_DIR="/opt/interview-assistant"
BIN_NAME="interview-assistant"

# Проверяем, что бинарник собран.
BIN_SRC="${INSTALL_DIR}/target/release/${BIN_NAME}"
if [ ! -f "$BIN_SRC" ]; then
    echo "Бинарник не найден: $BIN_SRC"
    echo "Сначала запусти install.sh для сборки проекта."
    exit 1
fi

# Чистим и создаём сборочную папку.
log "Создаю структуру пакета..."
rm -rf "$BUILD_DIR"
mkdir -p "${BUILD_DIR}/DEBIAN"
mkdir -p "${BUILD_DIR}/usr/local/bin"
mkdir -p "${BUILD_DIR}/usr/share/applications"
mkdir -p "${BUILD_DIR}/usr/share/icons/hicolor/256x256/apps"

# Копируем бинарник.
cp "$BIN_SRC" "${BUILD_DIR}/usr/local/bin/${BIN_NAME}"
chmod 755 "${BUILD_DIR}/usr/local/bin/${BIN_NAME}"

# Ярлык рабочего стола.
cat > "${BUILD_DIR}/usr/share/applications/${PKG_NAME}.desktop" <<EOF
[Desktop Entry]
Name=Interview Assistant
Comment=Offline speech + AI assistant for technical interviews
Exec=/usr/local/bin/${BIN_NAME}
Icon=${PKG_NAME}
Terminal=false
Type=Application
Categories=Utility;Office;
StartupWMClass=interview-assistant
EOF

# Метаданные пакета (control).
cat > "${BUILD_DIR}/DEBIAN/control" <<EOF
Package: ${PKG_NAME}
Version: ${PKG_VER}
Section: utils
Priority: optional
Architecture: ${PKG_ARCH}
Depends: libasound2, libgtk-3-0, libx11-6, libxcb1, libxcb-shape0, libxcb-xfixes0, libxkbcommon0, libwayland-client0
Recommends: vosk-model
Maintainer: KatoS <kato@example.com>
Description: Offline speech + AI assistant for technical interviews
 Interview Assistant captures system audio or microphone,
 transcribes speech offline via VOSK, and sends the text
 to an LLM (OpenAI / DeepSeek / OpenRouter / Ollama / GigaChat)
 for real-time interview assistance.
 .
 Features:
  - Offline STT (VOSK)
  - Multiple AI providers
  - Global hotkeys (arrow keys)
  - Dark glass theme (egui)
  - History of Q&A pairs
EOF

# Собираем .deb.
log "Собираю ${DEB_NAME}..."
dpkg-deb --build "$BUILD_DIR" "$DEB_NAME"

# Готово.
log "Пакет создан: $(pwd)/${DEB_NAME}"
echo ""
echo "Установка на целевой машине:"
echo "  sudo dpkg -i ${DEB_NAME}"
echo "  sudo apt-get install -f   # если не хватает зависимостей"
echo ""
echo "Потом скачай VOSK-модель и настрой в приложении."
