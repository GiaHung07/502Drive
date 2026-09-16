# 502Drive

502Drive is a local-first Telegram bot for cloning Google Drive files and folders, then optionally watching source folders for future changes.

It runs on your own machine with Telegram long polling, local Google OAuth, SQLite state, and Google Drive API copy operations. It does not require a VPS, webhook, public domain, Docker, or service-account rotation.

[Đọc bản tiếng Việt](README.vi.md)

## Features

- Clone a Google Drive file or folder from a pasted URL.
- Browse My Drive and Shared Drives from Telegram.
- Save and switch default destination folders.
- Track clone jobs with pause, resume, cancel, retry, and JSON/CSV reports.
- Open a button-driven Telegram control center with `/menu`.
- Watch source folders and apply future changes into destination folders.
- Use Vietnamese or English Telegram UI with `telegram.language = "vi"` or `"en"`.

## Status

Ready for local testing on Linux/CachyOS:

- OAuth login.
- One-shot clone.
- Destination picker.
- Job controls.
- Reports.
- Telegram control center.
- Linux `systemd --user` service.

Still being hardened before a polished public release:

- Full i18n for every Telegram message body and error.
- Watch reconciliation after long downtime.
- Windows end-to-end verification.
- Public release binaries and packaging.

## Why Local-First?

Most popular Telegram mirror/leech bots are built for VPS or Docker because they download torrents, direct links, archives, and media, then upload to cloud storage.

502Drive is narrower on purpose. It focuses on Google Drive-to-Google Drive clone/watch through the user's own Google account.

Local-first is the best default because:

- Google OAuth loopback login is simpler and safer on the user's machine.
- No public webhook, domain, TLS, or reverse proxy is needed.
- SQLite state and reports stay local.
- A user service can survive restarts without renting a server.

Docker Compose can be added later for NAS and homelab users, but it should stay optional until OAuth, config, backup, and volume mounting are well documented.

## Similar Projects

502Drive overlaps with existing Telegram Drive tools, but the product shape is different.

Existing public projects usually fall into these groups:

- Mirror/leech bots: download torrents, direct links, YouTube, archives, Telegram files, and Google Drive links, then upload to Google Drive, Telegram, or rclone remotes.
- Rclone Telegram bots: expose cloud-to-cloud transfers through rclone, usually Docker/VPS-first.
- Drive uploader bots: upload Telegram files or direct links into Google Drive.
- Clone bots: often built around Shared Drives and service accounts.

502Drive intentionally chooses a narrower lane:

- Google Drive-to-Google Drive clone/watch.
- Personal OAuth.
- Local SQLite durability.
- Telegram dashboard UX.
- Vietnamese and English UI.
- No quota bypass or service-account rotation.

There are many nearby repos but few direct matches because Drive watch/sync needs durable cursors, mapping, idempotency, and reconciliation. The larger Telegram bot community mostly optimizes for broad mirror/leech features and easy VPS/Docker deployment.

## Roadmap By Phase

1. Public baseline: license, contributing guide, issue templates, and CI.
2. Full i18n: every Telegram message, prompt, button, command, and user-facing error in Vietnamese and English.
3. UX shell: one editable control-center message, Home/Back/Refresh everywhere, and confirmation screens for destructive actions.
4. Clone hardening: better doctor checks, restart recovery, report summaries, and backup/restore docs.
5. Watch stable: reconciliation command, backlog health, manual confirmation workflow, and search/filter.
6. Packaging: Linux release binaries, Windows verification, and optional Docker Compose for NAS/homelab.

## Quick Start

Copy the sample config:

```bash
mkdir -p ~/.config/gdclone-bot
cp config.sample.toml ~/.config/gdclone-bot/config.toml
```

Edit the required values:

```toml
[telegram]
bot_token = "..."
owner_telegram_id = 123456789
language = "vi" # vi | en

[google_oauth]
client_id = "..."
client_secret = "..."
```

Log in and check the setup:

```bash
cargo run -- auth login
cargo run -- doctor
```

`doctor` reports what is ready and what still needs setup: Telegram, OAuth,
Google login/token refresh, Drive account, database, reports, and default
destination.

Run the bot:

```bash
cargo run -- run
```

In Telegram, send:

```text
/menu
```

## Telegram UX

Most daily work should be done by tapping buttons:

- Home: Google account, destination, jobs, and watches.
- Jobs: list, details, pause, resume, cancel.
- Destination: recent folders, My Drive browser, Shared Drive browser.
- Watches: list, details, pause, resume, stop, content update policy.

The bot should prefer one editable control-center message instead of sending a new menu message for every tap. Inline keyboards and message editing keep the chat from being pushed upward by command spam.

Watch policy buttons:

- `Tao ban moi` / `Versioned`: keep the old copy and create a new one.
- `Thay ban cu` / `Replace`: copy the new version, then trash the old copy.
- `Xac nhan tay` / `Manual`: stop for manual review.

The live Vietnamese Telegram UI uses proper Vietnamese labels with accents.

## Development

Run:

```bash
cargo fmt --check
cargo test
cargo build --release
```

Before opening a PR, also check:

- New Telegram text supports Vietnamese and English.
- New destructive actions have a confirmation step.
- New Drive writes have durable operation state before the API request.
- New settings are documented in `config.sample.toml`.

## Safety

502Drive does not bypass Google Drive access rules. It does not scrape, rotate service accounts, bypass download-disabled files, or evade quota limits.

Do not commit:

- `config.toml`
- OAuth tokens
- `master.key`
- `*.db`, `*.db-wal`, `*.db-shm`
- logs
- reports

## Authors

- Author: PGH
- Project/team: LanManTeam

## Docs

- [Vietnamese README](README.vi.md)
- [Architecture](docs/architecture.md)
- [Google OAuth setup](docs/google-oauth-setup.md)
- [CachyOS setup](docs/cachyos-setup.md)
- [Windows setup](docs/windows-setup.md)
- [Watch semantics](docs/watch-semantics.md)
- [Troubleshooting](docs/troubleshooting.md)
- [Security](SECURITY.md)
- [Authors](AUTHORS.md)
- [Notice](NOTICE.md)

## License

502Drive is licensed under the GNU General Public License, version 3 only (`GPL-3.0-only`).

Copyright (C) 2026 PGH / LanManTeam.

See [LICENSE](LICENSE) for the full GPLv3 text.
