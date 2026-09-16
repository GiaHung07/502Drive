#!/usr/bin/env bash
# install-systemd-user.sh — install gdclone-bot as a systemd user service.
# Re-run to upgrade the binary in place.
#
# Usage:
#   bash scripts/install-systemd-user.sh
#
# Environment overrides:
#   BINARY_PATH   — path to install binary (default: ~/.local/bin/gdclone-bot)
#   CONFIG_PATH   — path to config.toml (default: ~/.config/gdclone-bot/config.toml)

set -euo pipefail
BOLD='\033[1m'; GREEN='\033[32m'; RED='\033[31m'; RESET='\033[0m'
say()  { printf "${BOLD}==> %s${RESET}\n" "$*"; }
ok()   { printf "${GREEN}    ok:${RESET} %s\n" "$*"; }
fail() { printf "${RED}    ERROR:${RESET} %s\n" "$*" >&2; exit 1; }

SERVICE_NAME="gdclone-bot"
SERVICE_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/gdclone-bot"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/gdclone-bot"
LOG_DIR="${XDG_STATE_HOME:-$HOME/.local/state}/gdclone-bot"
BINARY_PATH="${BINARY_PATH:-$HOME/.local/bin/gdclone-bot}"
CONFIG_PATH="${CONFIG_PATH:-$CONFIG_DIR/config.toml}"

# ── 1. Binary ────────────────────────────────────────────────────────────────
say "Checking binary"
if [[ -f "target/release/$SERVICE_NAME" ]]; then
    say "Installing release binary → $BINARY_PATH"
    mkdir -p "$(dirname "$BINARY_PATH")"
    install -m 755 "target/release/$SERVICE_NAME" "$BINARY_PATH"
    ok "$BINARY_PATH installed"
elif [[ -f "$BINARY_PATH" ]]; then
    ok "Using existing binary at $BINARY_PATH"
else
    fail "Binary not found. Run: cargo build --release"
fi

# ── 2. Config ────────────────────────────────────────────────────────────────
say "Checking config"
if [[ ! -f "$CONFIG_PATH" ]]; then
    if [[ -f "config.sample.toml" ]]; then
        say "Copying sample config → $CONFIG_PATH"
        mkdir -p "$CONFIG_DIR"
        install -m 600 config.sample.toml "$CONFIG_PATH"
        ok "Edit $CONFIG_PATH and set bot_token and owner_telegram_id"
    else
        fail "Config not found at $CONFIG_PATH — create it first."
    fi
else
    ok "Config found at $CONFIG_PATH"
    # Warn if permissions are too open.
    perm=$(stat -c '%a' "$CONFIG_PATH" 2>/dev/null || stat -f '%Lp' "$CONFIG_PATH" 2>/dev/null || echo "???")
    if [[ "$perm" != "600" && "$perm" != "400" ]]; then
        printf "    \033[33mWARN:\033[0m config.toml permissions are %s — consider chmod 600\n" "$perm"
    fi
fi

# ── 3. Directories ──────────────────────────────────────────────────────────
say "Creating data/log directories"
mkdir -p "$DATA_DIR" "$CONFIG_DIR" "$LOG_DIR"
chmod 700 "$DATA_DIR" "$CONFIG_DIR"
ok "Directories ready"

# ── 4. Write unit file ──────────────────────────────────────────────────────
say "Writing systemd unit → $SERVICE_DIR/$SERVICE_NAME.service"
mkdir -p "$SERVICE_DIR"
cat > "$SERVICE_DIR/$SERVICE_NAME.service" <<EOF
[Unit]
Description=gdclone-bot — Google Drive clone/watch Telegram bot
Wants=network-online.target
After=network-online.target

[Service]
Type=simple
ExecStart=$BINARY_PATH --config $CONFIG_PATH run
Restart=on-failure
RestartSec=10s
RestartSteps=3
RestartMaxDelaySec=60s
TimeoutStartSec=30
TimeoutStopSec=30
StandardOutput=journal
StandardError=journal
SyslogIdentifier=gdclone-bot
Environment=RUST_LOG=gdclone_bot=info,warn

NoNewPrivileges=yes
PrivateTmp=yes
ProtectSystem=strict
ProtectHome=read-only
ProtectKernelTunables=yes
ProtectControlGroups=yes
RestrictSUIDSGID=yes
LockPersonality=yes
MemoryDenyWriteExecute=yes
RestrictRealtime=yes
SystemCallFilter=@system-service
SystemCallErrorNumber=EPERM

ReadWritePaths=$DATA_DIR
ReadWritePaths=$CONFIG_DIR
ReadWritePaths=$LOG_DIR

[Install]
WantedBy=default.target
EOF
ok "Unit file written"

# ── 5. Stop any foreground process ─────────────────────────────────────────
if pgrep -u "$USER" -f "$SERVICE_NAME .* run" > /dev/null 2>&1; then
    say "Stopping foreground gdclone-bot process"
    pkill -u "$USER" -f "$SERVICE_NAME .* run" || true
    sleep 1
fi

# ── 6. Enable lingering so service survives logout ─────────────────────────
say "Enabling user lingering (loginctl)"
loginctl enable-linger "$USER" 2>/dev/null || true

# ── 7. Enable and start ────────────────────────────────────────────────────
say "Reloading systemd user daemon"
systemctl --user daemon-reload

say "Enabling $SERVICE_NAME"
systemctl --user enable "$SERVICE_NAME.service"

say "Starting $SERVICE_NAME"
if systemctl --user start "$SERVICE_NAME.service"; then
    ok "Service started"
else
    printf "    \033[33mWARN:\033[0m start may have failed — check status below\n"
fi

echo ""
say "Status:"
systemctl --user status "$SERVICE_NAME.service" --no-pager -l || true

echo ""
say "Useful commands:"
echo "  journalctl --user -u $SERVICE_NAME -f          # follow logs"
echo "  systemctl --user status $SERVICE_NAME          # status"
echo "  systemctl --user restart $SERVICE_NAME         # restart"
echo "  systemctl --user stop $SERVICE_NAME            # stop"
echo "  $BINARY_PATH auth login                        # Google auth"
