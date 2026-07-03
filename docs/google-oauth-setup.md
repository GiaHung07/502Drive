# Google OAuth Setup

1. Create an OAuth desktop client in Google Cloud Console.
2. Configure the consent screen for a local personal app.
3. Put `client_id` and `client_secret` in `$HOME/.config/gdclone-bot/config.toml` or environment variables.
4. Run:

```bash
gdclone-bot --config "$HOME/.config/gdclone-bot/config.toml" auth login
gdclone-bot --config "$HOME/.config/gdclone-bot/config.toml" doctor
gdclone-bot --config "$HOME/.config/gdclone-bot/config.toml" run
```

The CLI flow opens a browser on the same machine when possible. If no desktop session is available, it prints the URL. Telegram `/connect` is only guidance; it does not complete OAuth.
