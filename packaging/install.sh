#!/usr/bin/env bash
# ==============================================================================
# 502Drive — Universal Linux Installer
# Local-first Telegram Bot for Google Drive Cloning & Realtime Sync
#
# Usage:
#   Local: bash packaging/install.sh
# ==============================================================================
set -euo pipefail

BOLD='\033[1m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
RESET='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

info() { printf "  ${CYAN}..${RESET} %s\n" "$1"; }
ok()   { printf "  ${GREEN}ok${RESET}: %s\n" "$1"; }
warn() { printf "  ${YELLOW}warn${RESET}: %s\n" "$1"; }
error_exit() { printf "\n  ${RED}error${RESET}: %s\n" "$1" >&2; exit 1; }

echo -e "${BOLD}==========================================================${RESET}"
echo -e "${BOLD}       502Drive — Linux User Service & Tray Installer     ${RESET}"
echo -e "${BOLD}==========================================================${RESET}"

BIN_PATH=""
if [ -f "$REPO_DIR/target/release/502drive" ]; then
    BIN_PATH="$REPO_DIR/target/release/502drive"
elif [ -f "$REPO_DIR/502drive" ]; then
    BIN_PATH="$REPO_DIR/502drive"
elif [ -f "$SCRIPT_DIR/502drive" ]; then
    BIN_PATH="$SCRIPT_DIR/502drive"
elif [ -f "$HOME/.local/bin/502drive" ]; then
    BIN_PATH="$HOME/.local/bin/502drive"
else
    warn "Binary '502drive' not found in release folder. Building now..."
    cargo build --release --bin 502drive
    BIN_PATH="$REPO_DIR/target/release/502drive"
fi

# 1. Install binary to ~/.local/bin
mkdir -p "$HOME/.local/bin"
cp "$BIN_PATH" "$HOME/.local/bin/502drive.new"
mv "$HOME/.local/bin/502drive.new" "$HOME/.local/bin/502drive"
chmod +x "$HOME/.local/bin/502drive"
ln -sf "$HOME/.local/bin/502drive" "$HOME/.local/bin/gdclone-bot"

# 1b. Install GUI binary if present
GUI_PATH=""
if [ -f "$REPO_DIR/target/release/502drive-gui" ]; then
    GUI_PATH="$REPO_DIR/target/release/502drive-gui"
elif [ -f "$REPO_DIR/target/release/drive502-gui" ]; then
    GUI_PATH="$REPO_DIR/target/release/drive502-gui"
fi
if [ -n "$GUI_PATH" ]; then
    cp "$GUI_PATH" "$HOME/.local/bin/502drive-gui.new"
    mv "$HOME/.local/bin/502drive-gui.new" "$HOME/.local/bin/502drive-gui"
    chmod +x "$HOME/.local/bin/502drive-gui"
    ln -sf "$HOME/.local/bin/502drive-gui" "$HOME/.local/bin/drive502-gui"
    ok "Installed desktop GUI to ~/.local/bin/502drive-gui"
fi
ok "Installed executable to ~/.local/bin/502drive"

# 2. Install tray applet
TRAY_SRC="$REPO_DIR/scripts/502drive-tray.py"
if [ -f "$TRAY_SRC" ]; then
    cp "$TRAY_SRC" "$HOME/.local/bin/502drive-tray"
    chmod +x "$HOME/.local/bin/502drive-tray"
    ln -sf "$HOME/.local/bin/502drive-tray" "$HOME/.local/bin/drive502-tray"
    ok "Installed System Tray applet to ~/.local/bin/502drive-tray"
fi

# 3. Install scalable & colored icons
ICON_DIR="$HOME/.local/share/icons/hicolor/scalable/apps"
mkdir -p "$ICON_DIR"
cp "$SCRIPT_DIR/502drive.svg" "$ICON_DIR/502drive.svg"
if [ -f "$SCRIPT_DIR/502drive.png" ]; then
    mkdir -p "$HOME/.local/share/pixmaps"
    cp "$SCRIPT_DIR/502drive.png" "$HOME/.local/share/pixmaps/502drive.png"
fi
cp "$SCRIPT_DIR/502drive-symbolic.svg" "$ICON_DIR/502drive-symbolic.svg"
cp "$SCRIPT_DIR/502drive-inactive-symbolic.svg" "$ICON_DIR/502drive-inactive-symbolic.svg"

# Install PNG icons if present
for s in 16 24 32 48 64 128 256 512; do
    PNG_DIR="$HOME/.local/share/icons/hicolor/${s}x${s}/apps"
    mkdir -p "$PNG_DIR"
    if [ -f "$SCRIPT_DIR/icons/${s}x${s}/502drive.png" ]; then
        cp "$SCRIPT_DIR/icons/${s}x${s}/502drive.png" "$PNG_DIR/502drive.png"
    fi
done

if command -v gtk-update-icon-cache &>/dev/null; then
    gtk-update-icon-cache -f "$HOME/.local/share/icons/hicolor" || true
fi
ok "Installed icons to ~/.local/share/icons/hicolor/"

# 4. Install desktop entry
APP_DIR="$HOME/.local/share/applications"
mkdir -p "$APP_DIR"
cp "$SCRIPT_DIR/502drive.desktop" "$APP_DIR/502drive.desktop"
if command -v update-desktop-database &>/dev/null; then
    update-desktop-database "$APP_DIR" || true
fi
ok "Installed desktop launcher to ~/.local/share/applications/502drive.desktop"

# 5. Install systemd user services
SYSTEMD_USER_DIR="$HOME/.config/systemd/user"
mkdir -p "$SYSTEMD_USER_DIR"
cp "$SCRIPT_DIR/gdclone-bot.service" "$SYSTEMD_USER_DIR/gdclone-bot.service"
cp "$SCRIPT_DIR/502drive-tray.service" "$SYSTEMD_USER_DIR/502drive-tray.service"
ok "Installed systemd user services to ~/.config/systemd/user/"

# 6. Setup default config if absent
CONFIG_DIR="$HOME/.config/gdclone-bot"
mkdir -p "$CONFIG_DIR"
if [ ! -f "$CONFIG_DIR/config.toml" ]; then
    if [ -f "$REPO_DIR/config.sample.toml" ]; then
        cp "$REPO_DIR/config.sample.toml" "$CONFIG_DIR/config.toml"
        info "Created default config at ~/.config/gdclone-bot/config.toml"
        warn "Vui lòng mở ~/.config/gdclone-bot/config.toml để điền Telegram Bot Token và Google Client ID!"
    fi
else
    ok "Existing configuration file found at ~/.config/gdclone-bot/config.toml"
fi

# 7. Reload systemd daemon
if command -v systemctl &>/dev/null; then
    systemctl --user daemon-reload || true
    ok "Systemd daemon reloaded"
fi

echo ""
echo -e "${GREEN}${BOLD}Cài đặt hoàn tất!${RESET}"
echo -e "Để khởi động dịch vụ:"
echo -e "  ${CYAN}systemctl --user enable --now gdclone-bot.service${RESET}"
echo -e "  ${CYAN}systemctl --user enable --now 502drive-tray.service${RESET}"
echo -e "Xem nhật ký:"
echo -e "  ${CYAN}journalctl --user -u gdclone-bot.service -f${RESET}"
