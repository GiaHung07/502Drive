# Privacy Policy — 502Drive

**Last updated:** September 2026

## 1. Overview
502Drive ("the App") is an open-source Google Drive clone/sync desktop application (a local daemon with Telegram bot control and a desktop GUI). This Privacy Policy describes how 502Drive handles your data.

## 2. Data the App stores, and where

502Drive stores the following data **locally on your device only**:

- **Google OAuth Tokens**: Access and refresh tokens required to authorize Google Drive API calls. The refresh token is encrypted using AES-256-GCM and stored in a local SQLite database (`~/.local/share/gdclone-bot/state.db` on Linux, `%APPDATA%\gdclone-bot\data\` on Windows). The encryption key is a locally generated `master.key` file (32 random bytes, permissions `0600` on Unix) in the config directory — it is **not** stored in an OS keyring.
- **Telegram Bot Token & Owner ID**: Used to receive commands from your own private Telegram bot. Stored in `~/.config/gdclone-bot/config.toml` (or the equivalent per-platform config path / `GDCLONE__` environment variables).
- **Job History & File Metadata**: Metadata (file names, IDs, sizes) of files processed during copy/sync operations. Stored locally in the same SQLite database.

**Data flow, stated explicitly:** OAuth tokens and Drive file metadata live only in the local SQLite database; **file contents never transit the App at all** — copies are performed server-side by Google Drive API operations directly between your source and destination folders. The App talks only to Google's API endpoints and Telegram's API endpoint (outbound connections; it listens only on loopback during OAuth login). It operates no servers.

## 3. Data We Do NOT Collect
- We **do not** collect, transmit, or store any of your data on remote servers.
- We **do not** have access to your Google account credentials (passwords).
- We **do not** sell, share, or transmit your personal information to any third party.
- We **do not** track usage analytics or telemetry.

## 4. Google Drive API Usage
502Drive uses the Google Drive API (`https://www.googleapis.com/auth/drive`) solely to:
- List, copy, and sync files between folders in your Google Drive on your behalf.
- Refresh access tokens automatically when expired.

All Google Drive API calls are made **directly from your machine** to Google's servers. No data passes through 502Drive's servers (there are none). Because the App is BYOK ("bring your own keys"), you create and control the Google Cloud OAuth client yourself — see [docs/oauth-setup.md](docs/oauth-setup.md).

## 5. Data Retention
All locally stored data (tokens, job history, file metadata) remains on your device. You can delete all data at any time by:
- Running `502drive auth revoke` to delete Google tokens.
- Deleting the data directory manually (`~/.local/share/gdclone-bot/` on Linux, `%APPDATA%\gdclone-bot\data\` on Windows) together with `master.key` in the config directory.
- Uninstalling the application.

## 6. Security
- OAuth refresh tokens are encrypted at rest using AES-256-GCM with a per-device master key stored as a **local `master.key` file** (`0600` permissions on Unix) in the config directory. The App does not use an OS keyring.
- SQLite data files are created with `0600` permissions and parent directories with `0700` on Unix.
- The App never requests or stores your Google account password.
- See [SECURITY.md](SECURITY.md) for vulnerability reporting and [docs/threat-model.md](docs/threat-model.md) for the full control list.

## 7. Third-Party Services
- **Google Drive API**: Governed by [Google's Privacy Policy](https://policies.google.com/privacy).
- **Telegram Bot API**: Governed by [Telegram's Privacy Policy](https://telegram.org/privacy). Used only to receive commands from your own private bot.

## 8. Changes to This Policy
We may update this Privacy Policy as the application evolves. Changes will be committed to this repository and the "Last updated" date will reflect the latest revision.

## 9. Contact
For questions about this Privacy Policy, please open an issue on our GitHub repository:
https://github.com/GiaHung07/502Drive/issues
