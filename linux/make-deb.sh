#!/bin/bash
# make-deb.sh — create a .deb installer package on Linux
# Usage: ./linux/make-deb.sh
# Output: interview-assistant_1.0.1_amd64.deb
set -euo pipefail

GREEN='\033[0;32m'; NC='\033[0m'
log() { echo -e "${GREEN}[+]${NC} $*"; }

PKG_NAME="interview-assistant"
PKG_VER="1.0.1"
PKG_ARCH="amd64"
DEB_NAME="${PKG_NAME}_${PKG_VER}_${PKG_ARCH}.deb"
BUILD_DIR="/tmp/${PKG_NAME}-deb"

# ── Clean and create build dir ────────────────────────────────
log "Creating package structure..."
rm -rf "$BUILD_DIR"
mkdir -p "${BUILD_DIR}/DEBIAN"
mkdir -p "${BUILD_DIR}/usr/local/bin"
mkdir -p "${BUILD_DIR}/usr/share/applications"
mkdir -p "${BUILD_DIR}/usr/share/${PKG_NAME}"

# ── Copy install script into package ──────────────────────────
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cp "${SCRIPT_DIR}/install.sh" "${BUILD_DIR}/usr/share/${PKG_NAME}/install.sh"
chmod 755 "${BUILD_DIR}/usr/share/${PKG_NAME}/install.sh"

# ── DEBIAN/control ────────────────────────────────────────────
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

# ── DEBIAN/postinst ───────────────────────────────────────────
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

log "Interview Assistant post-install setup..."

# ── Rust toolchain ────────────────────────────────────────────
if command -v rustc &>/dev/null; then
    log "Rust already installed: $(rustc --version)"
else
    log "Installing Rust via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
    source "$HOME/.cargo/env"
fi

export PATH="$HOME/.cargo/bin:$PATH"
rustup default stable 2>/dev/null || true

# ── Clone / update repo ───────────────────────────────────────
if [ -d "$INSTALL_DIR/.git" ]; then
    log "Updating existing repo..."
    cd "$INSTALL_DIR"
    git fetch origin "$BRANCH"
    git checkout "$BRANCH"
    git reset --hard "origin/$BRANCH"
else
    log "Cloning repo into $INSTALL_DIR..."
    mkdir -p "$INSTALL_DIR"
    git clone --branch "$BRANCH" --depth 1 "$REPO_URL" "$INSTALL_DIR"
fi

# ── Build ─────────────────────────────────────────────────────
cd "$INSTALL_DIR"
log "Building release binary (this may take a few minutes)..."
cargo build --release 2>&1 | tail -5

BIN_PATH="$INSTALL_DIR/target/release/$BIN_NAME"
if [ ! -f "$BIN_PATH" ]; then
    echo "ERROR: Build failed — binary not found at $BIN_PATH"
    exit 1
fi

# ── Install binary ────────────────────────────────────────────
log "Installing binary to /usr/local/bin/$BIN_NAME..."
cp "$BIN_PATH" "/usr/local/bin/$BIN_NAME"
chmod +x "/usr/local/bin/$BIN_NAME"

# ── Desktop entry ─────────────────────────────────────────────
log "Creating desktop entry..."
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

# ── Done ──────────────────────────────────────────────────────
echo ""
log "══════════════════════════════════════════════════════"
log " Interview Assistant installed successfully!"
log "══════════════════════════════════════════════════════"
echo ""
warn "IMPORTANT: You still need a VOSK model for speech recognition."
echo "  Download: https://alphacephei.com/vosk/models"
echo "  Extract it and set the path in the app's Settings tab."
echo ""
log "Launch: interview-assistant  (or find it in your app menu)"
echo ""
POSTINST
chmod 755 "${BUILD_DIR}/DEBIAN/postinst"

# ── Build .deb ────────────────────────────────────────────────
log "Building ${DEB_NAME}..."
dpkg-deb --build "$BUILD_DIR" "$DEB_NAME"

# ── Done ─────────────────────────────────────────────────────
log "Package created: $(pwd)/${DEB_NAME}"
echo ""
echo "Install on target machine:"
echo "  sudo dpkg -i ${DEB_NAME}"
echo ""
echo "Then download a VOSK model and configure the app."
