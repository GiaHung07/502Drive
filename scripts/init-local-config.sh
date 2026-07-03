#!/usr/bin/env bash
set -euo pipefail

CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/gdclone-bot"
CONFIG_PATH="$CONFIG_DIR/config.toml"
SAMPLE_PATH="${SAMPLE_PATH:-config.sample.toml}"

if [[ ! -f "$SAMPLE_PATH" ]]; then
  echo "ERROR: config sample not found: $SAMPLE_PATH" >&2
  exit 1
fi

mkdir -p "$CONFIG_DIR"
install -m 600 "$SAMPLE_PATH" "$CONFIG_PATH"

read -r -p "Telegram bot token: " BOT_TOKEN
read -r -p "Telegram owner user id: " OWNER_ID
read -r -p "Google OAuth client id: " GOOGLE_CLIENT_ID
read -r -s -p "Google OAuth client secret: " GOOGLE_CLIENT_SECRET
printf '\n'

CONFIG_PATH="$CONFIG_PATH" \
BOT_TOKEN="$BOT_TOKEN" \
OWNER_ID="$OWNER_ID" \
GOOGLE_CLIENT_ID="$GOOGLE_CLIENT_ID" \
GOOGLE_CLIENT_SECRET="$GOOGLE_CLIENT_SECRET" \
python - <<'PY'
import os
from pathlib import Path

path = Path(os.environ["CONFIG_PATH"])
text = path.read_text()
replacements = {
    'bot_token = "REPLACE_ME"': f'bot_token = "{os.environ["BOT_TOKEN"]}"',
    'owner_telegram_id = 0': f'owner_telegram_id = {os.environ["OWNER_ID"]}',
    'client_id = "REPLACE_ME.apps.googleusercontent.com"': f'client_id = "{os.environ["GOOGLE_CLIENT_ID"]}"',
    'client_secret = "REPLACE_ME"': f'client_secret = "{os.environ["GOOGLE_CLIENT_SECRET"]}"',
    'enabled = true': 'enabled = false',
}
for old, new in replacements.items():
    text = text.replace(old, new)
path.write_text(text)
PY

chmod 600 "$CONFIG_PATH"
echo "Wrote $CONFIG_PATH"
