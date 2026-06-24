#!/bin/bash
# Interview Assistant — Linux installer (Debian 12+ / Ubuntu 22.04+)
# Usage: chmod +x install.sh && ./install.sh
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

# ── Check distro ──────────────────────────────────────────────
if [ -f /etc/os-release ]; then
    . /etc/os-release
    log "Detected: $NAME $VERSION_ID"
    case "$ID" in
        debian) [ "${VERSION_ID%.*}" -ge 12 ] || warn "Debian <12 may miss some packages";;
        ubuntu) [ "${VERSION_ID%.*}" -ge 22 ] || warn "Ubuntu <22.04 may miss some packages";;
        *)      warn "Untested distro: $ID. Proceeding anyway.";;
    esac
else
    warn "Cannot detect distro. Proceeding anyway."
fi

# ── System dependencies ───────────────────────────────────────
log "Installing system dependencies..."
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

# ── Rust toolchain ────────────────────────────────────────────
if command -v rustc &>/dev/null; then
    RUST_VER=$(rustc --version | cut -d' ' -f2)
    log "Rust already installed: $RUST_VER"
else
    log "Installing Rust via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
    # shellcheck source=/dev/null
    source "$HOME/.cargo/env"
fi

# Ensure ~/.cargo/bin is in PATH for this session
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
    sudo mkdir -p "$INSTALL_DIR"
    sudo chown "$USER:$USER" "$INSTALL_DIR"
    git clone --branch "$BRANCH" --depth 1 "$REPO_URL" "$INSTALL_DIR"
fi

# ── Build ─────────────────────────────────────────────────────
cd "$INSTALL_DIR"
log "Building release binary (this may take a few minutes)..."
cargo build --release 2>&1 | tail -5

BIN_PATH="$INSTALL_DIR/target/release/$BIN_NAME"
if [ ! -f "$BIN_PATH" ]; then
    err "Build failed — binary not found at $BIN_PATH"
fi

# ── Install binary ────────────────────────────────────────────
log "Installing binary to /usr/local/bin/$BIN_NAME..."
sudo cp "$BIN_PATH" "/usr/local/bin/$BIN_NAME"
sudo chmod +x "/usr/local/bin/$BIN_NAME"

# ── Desktop entry ─────────────────────────────────────────────
log "Creating desktop entry..."
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

# ── VOSK model reminder ──────────────────────────────────────
echo ""
log "══════════════════════════════════════════════════════"
log " Installation complete!"
log "══════════════════════════════════════════════════════"
echo ""
warn "IMPORTANT: You still need a VOSK model for speech recognition."
echo "  Download one of:"
echo "    • Russian:  https://alphacephei.com/vosk/models/vosk-model-ru-0.10.zip  (~1.4 GB)"
echo "    • English:  https://alphacephei.com/vosk/models/vosk-model-en-us-0.22.zip (~1.8 GB)"
echo "    • More:     https://alphacephei.com/vosk/models"
echo ""
echo "  Extract it anywhere (e.g. ~/vosk-model-ru-0.10) and set the path"
echo "  in the app's Settings tab on first launch."
echo ""
log "Launch: interview-assistant  (or find it in your app menu)"
echo ""
