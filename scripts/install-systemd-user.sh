#!/usr/bin/env bash
# install-systemd-user.sh — install gdclone-bot as a systemd user service.
# Re-run to upgrade the binary in place.

set -euo pipefail
BOLD='\033[1m'; RESET='\033[0m'
say() { printf "${BOLD}==> %s${RESET}\n" "$*"; }

SERVICE_NAME="gdclone-bot"
SERVICE_DIR="$HOME/.config/systemd/user"
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/gdclone-bot"
BINARY_PATH="${BINARY_PATH:-$HOME/.local/bin/gdclone-bot}"
CONFIG_PATH="${CONFIG_PATH:-$CONFIG_DIR/config.toml}"

if [[ ! -f "$BINARY_PATH" ]]; then
  echo "ERROR: Binary not found at $BINARY_PATH"
  echo "Run scripts/setup-cachyos.sh first, or set BINARY_PATH."
  exit 1
fi

if [[ ! -f "$CONFIG_PATH" ]]; then
  echo "ERROR: Config not found at $CONFIG_PATH"
  echo "Create your config.toml first."
  exit 1
fi

mkdir -p "$SERVICE_DIR"

say "Writing systemd unit: $SERVICE_DIR/$SERVICE_NAME.service"
cat > "$SERVICE_DIR/$SERVICE_NAME.service" <<EOF
[Unit]
Description=gdclone-bot — Telegram Google Drive clone bot
Documentation=https://github.com/you/gdclone-bot
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=$BINARY_PATH --config $CONFIG_PATH run
Restart=on-failure
RestartSec=5s
StandardOutput=journal
StandardError=journal
Environment=RUST_LOG=gdclone_bot=info

[Install]
WantedBy=default.target
EOF

say "Reloading systemd user daemon"
systemctl --user daemon-reload

say "Enabling and starting $SERVICE_NAME"
systemctl --user enable --now "$SERVICE_NAME.service"

say "Service status:"
systemctl --user status "$SERVICE_NAME.service" --no-pager || true
echo ""
say "View logs: journalctl --user -u $SERVICE_NAME -f"
