#!/usr/bin/env bash
# uninstall-systemd-user.sh — stop and remove the systemd user service.

set -euo pipefail
BOLD='\033[1m'; RESET='\033[0m'
say() { printf "${BOLD}==> %s${RESET}\n" "$*"; }

SERVICE_NAME="gdclone-bot"

say "Stopping $SERVICE_NAME"
systemctl --user stop "$SERVICE_NAME.service" 2>/dev/null || true

say "Disabling $SERVICE_NAME"
systemctl --user disable "$SERVICE_NAME.service" 2>/dev/null || true

UNIT_FILE="$HOME/.config/systemd/user/$SERVICE_NAME.service"
if [[ -f "$UNIT_FILE" ]]; then
  say "Removing unit file: $UNIT_FILE"
  rm "$UNIT_FILE"
fi

systemctl --user daemon-reload
say "Done. Config and database are untouched — run install-systemd-user.sh to reinstall."
