#!/usr/bin/env bash
# restore.sh — restore gdclone-bot from a backup archive.
#
# Usage: ./scripts/restore.sh <backup.tar.gz>
#
# WARNING: This overwrites existing config and state. Stop the service first.

set -euo pipefail
BOLD='\033[1m'; RED='\033[31m'; RESET='\033[0m'
say()  { printf "${BOLD}==> %s${RESET}\n" "$*"; }
die()  { printf "${RED}ERROR: %s${RESET}\n" "$*" >&2; exit 1; }

ARCHIVE="${1:-}"
[[ -n "$ARCHIVE" ]] || die "Usage: $0 <backup.tar.gz>"
[[ -f "$ARCHIVE" ]] || die "Archive not found: $ARCHIVE"

say "Stopping gdclone-bot service (if running)"
systemctl --user stop gdclone-bot.service 2>/dev/null || true

say "Restoring from: $ARCHIVE"
# Extract to / (the archive contains absolute paths under ~/.config and ~/.local).
tar -xzf "$ARCHIVE" -C /

say "Restore complete."
say "Review config if needed, then restart:"
echo "  systemctl --user start gdclone-bot.service"
echo "  # or: gdclone-bot --config ~/.config/gdclone-bot/config.toml run"
