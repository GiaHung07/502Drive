# Security

Report security issues privately to the project owner.

Do not commit:

- `config.toml`
- `.env`
- `master.key`
- `*.db`
- `reports/`
- `logs/`

Rotate credentials:

1. Stop the bot.
2. Run `gdclone-bot auth revoke`.
3. Remove old local credential material only after backup.
4. Run `gdclone-bot auth login`.
5. Run `gdclone-bot doctor`.
