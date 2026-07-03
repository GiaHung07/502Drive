#!/usr/bin/env bash
# backup.sh — back up gdclone-bot state database and config.
# Creates a timestamped .tar.gz in the directory this script is called from
# (or $BACKUP_DIR if set).
#
# Usage: ./scripts/backup.sh [output_dir]

set -euo pipefail
BOLD='\033[1m'; RESET='\033[0m'
say() { printf "${BOLD}==> %s${RESET}\n" "$*"; }

BACKUP_DIR="${1:-${BACKUP_DIR:-$PWD}}"
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/gdclone-bot"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/gdclone-bot"

TIMESTAMP=$(date +%Y%m%dT%H%M%S)
ARCHIVE="$BACKUP_DIR/gdclone-bot-backup-$TIMESTAMP.tar.gz"

say "Creating backup: $ARCHIVE"

# Collect files that exist.
FILES=()
[[ -f "$CONFIG_DIR/config.toml" ]]  && FILES+=("$CONFIG_DIR/config.toml")
[[ -d "$DATA_DIR" ]]                && FILES+=("$DATA_DIR/")

if [[ ${#FILES[@]} -eq 0 ]]; then
  echo "Nothing to back up — no config or data directory found."
  exit 1
fi

mkdir -p "$BACKUP_DIR"
tar -czf "$ARCHIVE" "${FILES[@]}"

BYTES=$(stat -c%s "$ARCHIVE" 2>/dev/null || stat -f%z "$ARCHIVE")
say "Backup complete: $ARCHIVE ($BYTES bytes)"
say "Restore with: scripts/restore.sh $ARCHIVE"
