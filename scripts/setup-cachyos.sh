#!/usr/bin/env bash
# setup-cachyos.sh — one-shot setup for gdclone-bot on CachyOS / Arch Linux.
# Run as your normal user (not root). Requires: sudo, git, rustup (or pacman rust).

set -euo pipefail
BOLD='\033[1m'; RESET='\033[0m'
say() { printf "${BOLD}==> %s${RESET}\n" "$*"; }

# ── System packages ──────────────────────────────────────────────────────────
say "Installing system dependencies"
sudo pacman -S --needed --noconfirm \
  base-devel \
  sqlite \
  openssl \
  pkg-config \
  libsecret \
  gnome-keyring \
  xdg-utils

# ── Rust toolchain ───────────────────────────────────────────────────────────
if ! command -v rustup &>/dev/null; then
  say "Installing rustup"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
  # shellcheck disable=SC1090
  source "$HOME/.cargo/env"
fi

say "Updating Rust to stable"
rustup update stable

# ── Build ────────────────────────────────────────────────────────────────────
say "Building gdclone-bot (release)"
cargo build --release

BINARY="$(cargo locate-project --message-format plain | xargs dirname)/target/release/gdclone-bot"
INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"
install -m 755 "$BINARY" "$INSTALL_DIR/gdclone-bot"
say "Installed → $INSTALL_DIR/gdclone-bot"

# ── Config ───────────────────────────────────────────────────────────────────
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/gdclone-bot"
mkdir -p "$CONFIG_DIR"
if [[ ! -f "$CONFIG_DIR/config.toml" ]]; then
  say "Creating default config at $CONFIG_DIR/config.toml"
  if [[ -f "config.sample.toml" ]]; then
    cp config.sample.toml "$CONFIG_DIR/config.toml"
  else
    say "WARNING: config.sample.toml not found — copy config manually"
  fi
fi

say "Done! Next steps:"
echo "  1. Edit $CONFIG_DIR/config.toml (set bot_token, owner_telegram_id, oauth credentials)"
echo "  2. Run: gdclone-bot auth login"
echo "  3. Run: scripts/install-systemd-user.sh"
