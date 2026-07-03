# Troubleshooting

- `invalid_grant`: run `gdclone-bot auth login` again.
- `insufficientPermissions`: verify Drive permissions and destination folder capabilities.
- Telegram does not respond: verify bot token, `getMe`, and that no webhook is set.
- systemd user service exits after logout: consider `loginctl enable-linger "$USER"`.

