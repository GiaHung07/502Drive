# Troubleshooting

Common issues across Linux, Windows, and Docker. Binary name is `502drive` (`gdclone-bot` is an alias). See [install-linux.md](install-linux.md), [install-windows.md](install-windows.md), [install-docker.md](install-docker.md), [oauth-setup.md](oauth-setup.md).

## 1. `cargo build` fails: missing compiler or toolchain

Rust 1.85+ (2024 edition) is required. Check with `rustc --version` and update:

```bash
rustup update stable
cargo build --release --bin 502drive
```

There are no system C dependencies (pure-Rust TLS, bundled SQLite). On Debian/Ubuntu, install `build-essential` and `pkg-config` for a working C toolchain/linker.

## 2. Config file not found / parse error

The default path is `~/.config/gdclone-bot/config.toml` (desktop Linux), `%APPDATA%\gdclone-bot\config\config.toml` (Windows), or `--config <path>` / `/config/config.toml` (Docker). Startup fails with `read config ...` if the file is missing, or names the offending key if a value is invalid. Start from `config.sample.toml` and keep all sections — env vars (`GDCLONE__...`) only *override* values, they cannot replace the file. `502drive doctor` prints the paths in use.

## 3. systemd user unit does not start

```bash
systemctl --user status gdclone-bot --no-pager -l
journalctl --user -u gdclone-bot -n 50
```

- After editing/reinstalling the unit, run `systemctl --user daemon-reload` first.
- Service dies at logout: `loginctl enable-linger "$USER"` (the install script does this when permitted; on some hosts PAM/policy blocks linger for non-logged-in users).
- The hardened unit uses `ProtectSystem=strict` with `ReadWritePaths` limited to config/data/log dirs — if you moved `storage.db_path` elsewhere, add that path to `ReadWritePaths=` in `~/.config/systemd/user/gdclone-bot.service`.

## 4. Telegram: bot does not respond, HTTP 401 / 409

- **401 Unauthorized**: wrong `telegram.bot_token`. Verify the token with `502drive doctor` (it calls `getMe`), or regenerate via @BotFather.
- **409 Conflict**: another process is long-polling the same bot token (a second instance, or a webhook). Stop the duplicate; delete any webhook: `curl "https://api.telegram.org/bot<TOKEN>/deleteWebhook"`.
- Commands silently ignored: your Telegram ID is not the `owner_telegram_id` in the config — send `/whoami` to the bot to see your ID.

## 5. Google OAuth: `Error 403: access_denied`

The OAuth consent screen is **not published** (still in "Testing"), so Google rejects accounts that are not registered test users. In Google Cloud Console → APIs & Services → OAuth consent screen, click **Publish App** (no verification needed for personal use — see [oauth-setup.md](oauth-setup.md)). Then run `502drive auth login` again.

## 6. Google API 429 / rate limiting

Responses with HTTP 429 or reasons `rateLimitExceeded` / `userRateLimitExceeded` are retried automatically with exponential backoff and jitter (`engine.retry_base_delay_ms` → `retry_max_delay_ms`), and a shared token-bucket pacer with a circuit breaker throttles all Drive requests. If you still see persistent failures: lower `engine.initial_write_concurrency` / `engine.max_write_concurrency`, and lower `engine.max_active_jobs`. Large initial clones of huge trees will always take a while — check `/preview` for live worker status.

## 7. OAuth login hangs or port conflicts (51000–51100)

`auth login` binds `127.0.0.1` on a free port in the `redirect_port_start`–`redirect_port_end` range. If every port in the range is taken (rare) or a firewall/proxy intercepts loopback traffic, login times out:

- Find the squatter: `ss -tlnp | grep 510` (Linux) / `netstat -ano | findstr 510` (Windows), stop it or widen the range in `config.toml`.
- On remote/headless setups the browser must be able to reach the listener — see [install-docker.md](install-docker.md) §4 (authenticate on a desktop and copy state, or publish the port range).
- The listener is loopback-only; a reverse proxy on the host does not interfere.

## 8. `invalid_grant` / token errors after a clock jump

`502drive auth login` again to refresh the stored token. `invalid_grant` also appears when the system clock is wrong (VM resume, dual-boot, wrong TZ on containers): verify the clock (`timedatectl`, `w32tm /query /status`, or correct `TZ` for Docker — the image defaults to `Asia/Ho_Cho_Minh`) and re-authenticate. Persistent clock skew also breaks token refresh during `run`; fix the time source rather than re-logging in repeatedly.

## 9. SQLite: `database is locked` / DB corruption

The DB is opened in WAL mode with a busy timeout, and all access goes through one connection actor, so lock errors indicate a second process using the same `state.db` with different settings, or a stuck process:

- Make sure the daemon and any manual CLI invocations point at the same config (`--config`), and that only one daemon runs (`pgrep -af 502drive`).
- Check integrity: `502drive doctor` prints the SQLite `integrity_check` result.
- After a hard crash, leftover `state.db-wal` / `state.db-shm` files are replayed automatically by SQLite; do not delete them manually. Take a backup with `502drive backup <output_dir>` before any manual intervention (also see `scripts/backup.sh`).

## 10. Desktop GUI cannot find the service's data

The Tauri GUI (`502drive-gui`) and the systemd service must read the **same** `config.toml`/`state.db`:

- The service unit passes `--config ~/.config/gdclone-bot/config.toml`; if the GUI was launched with a different config (or different `storage.db_path`), it opens a different DB and shows no jobs/watches. Point both at the same config.
- With the hardened systemd unit, `ProtectHome=read-only` still lets other processes read the DB; ensure your user can read `~/.local/share/gdclone-bot/state.db` (`ls -l`, expect `0600` owned by you).
- Never run two writers against the same DB (GUI writing while the daemon runs); use the Telegram interface for job control while the service is active. See issue 9 for lock behavior.

## 11. Watch events stop applying / "needs_reconcile" notifications

The watch hit its backlog limit (`watch.max_backlog_events_per_watch`) or encountered an ambiguous event; it pauses applying until reconciled. Inspect with `/watch_status <watch_id>` in Telegram, then resume with `/watch_resume`. Background: [sync-semantics.md](sync-semantics.md).

## 12. Windows scheduled task runs but nothing happens

Check `%APPDATA%\gdclone-bot\data\logs\` for the JSON log. Common causes: the task runs under a different Windows user than the one that did `auth login` (tokens and `master.key` are per-user), or the config path passed to the task does not exist. Keep OAuth login and the task under the same user — see [install-windows.md](install-windows.md).
