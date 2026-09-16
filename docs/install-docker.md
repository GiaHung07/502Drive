# Docker Installation

The image is a multi-stage build (`Dockerfile`): `rust:bookworm` compiles `--bin 502drive`, and the runtime is `debian:bookworm-slim` running as non-root user `appuser` (UID 10001). Only `ca-certificates`, `tzdata`, and `curl` are installed in the runtime layer. See [install-linux.md](install-linux.md) / [install-windows.md](install-windows.md) for native installs.

## 1. Quick start with Docker Compose

```bash
git clone https://github.com/GiaHung07/502Drive.git
cd 502Drive
mkdir -p config data
cp config.sample.toml config/config.toml
$EDITOR config/config.toml          # see §2
docker compose up -d
docker compose logs -f
```

The provided `docker-compose.yml`:

- mounts `./config` → `/config` and `./data` → `/data` (both are declared volumes in the image);
- sets `restart: unless-stopped`;
- publishes `51000-51010:51000-51010` for the OAuth loopback redirect (§4).

The container entrypoint is `502drive --config /config/config.toml run`, so the config file must exist at `./config/config.toml` on the host.

## 2. Configuration

Docker deployments read `/config/config.toml`. Configure `telegram.bot_token`, `telegram.owner_telegram_id`, `google_oauth.client_id`, and `google_oauth.client_secret` either in that file or via environment variables (§3). Keep `watch.enabled = false` unless you want change polling; sync semantics are documented in [sync-semantics.md](sync-semantics.md).

State (SQLite DB, logs, reports) should be kept on the `/data` volume — see the persistence note in §3, because the built-in path defaults point inside the container user's home instead.

## 3. Headless setup via `GDCLONE__` environment variables

Any config key listed below can be set as an environment variable instead of editing the TOML. Precedence: env var overrides `config.toml` (implemented in `src/config.rs`, `apply_env_overrides`). The separator is a double underscore; section/key names are uppercase.

| Environment variable | Config key | Purpose |
| :--- | :--- | :--- |
| `GDCLONE__TELEGRAM__BOT_TOKEN` | `telegram.bot_token` | Token from @BotFather |
| `GDCLONE__TELEGRAM__OWNER_TELEGRAM_ID` | `telegram.owner_telegram_id` | Your Telegram user ID (owner) |
| `GDCLONE__TELEGRAM__PROGRESS_EDIT_MIN_INTERVAL_MS` | `telegram.progress_edit_min_interval_ms` | Throttle for progress message edits |
| `GDCLONE__TELEGRAM__LANGUAGE` | `telegram.language` | `vi` or `en` |
| `GDCLONE__DESTINATION__AUTO_CONFIRM_CLONE` | `destination.auto_confirm_clone` | `true`/`false` |
| `GDCLONE__DESTINATION__WRAP_SINGLE_FILE_IN_FOLDER` | `destination.wrap_single_file_in_folder` | `true`/`false` |
| `GDCLONE__GOOGLE_OAUTH__CLIENT_ID` | `google_oauth.client_id` | OAuth Desktop client ID |
| `GDCLONE__GOOGLE_OAUTH__CLIENT_SECRET` | `google_oauth.client_secret` | OAuth client secret |
| `GDCLONE__GOOGLE_OAUTH__SCOPE` | `google_oauth.scope` | Drive scope (default `https://www.googleapis.com/auth/drive`) |
| `GDCLONE__ENGINE__MAX_ACTIVE_JOBS` | `engine.max_active_jobs` | Concurrent job limit |
| `GDCLONE__ENGINE__MAX_ACTIVE_JOBS_PER_USER` | `engine.max_active_jobs_per_user` | Per-user job limit |
| `GDCLONE__ENGINE__INITIAL_WRITE_CONCURRENCY` | `engine.initial_write_concurrency` | Initial copy workers |
| `GDCLONE__ENGINE__MAX_RETRY_ATTEMPTS` | `engine.max_retry_attempts` | Per-write retry cap |
| `GDCLONE__ENGINE__RETRY_BASE_DELAY_MS` | `engine.retry_base_delay_ms` | Backoff base |
| `GDCLONE__ENGINE__RETRY_MAX_DELAY_MS` | `engine.retry_max_delay_ms` | Backoff cap |
| `GDCLONE__ENGINE__REQUEST_TIMEOUT_SECONDS` | `engine.request_timeout_seconds` | Drive request timeout |
| `GDCLONE__STORAGE__DB_PATH` | `storage.db_path` | SQLite path |
| `GDCLONE__STORAGE__LOG_DIR` | `storage.log_dir` | Log directory |
| `GDCLONE__STORAGE__REPORT_DIR` | `storage.report_dir` | Report directory |

> **Important (state persistence):** with the default empty `storage.*` values, paths resolve relative to the container user's home (`/home/appuser/.local/share/gdclone-bot/...`), which is **not** on the mounted `/data` volume and is lost when the container is recreated. For persistent state, set them explicitly in `config/config.toml`:
>
> ```toml
> [storage]
> db_path = "/data/state.db"
> log_dir = "/data/logs"
> report_dir = "/data/reports"
> ```
>
> or via `GDCLONE__STORAGE__DB_PATH=/data/state.db`, `GDCLONE__STORAGE__LOG_DIR=/data/logs`, `GDCLONE__STORAGE__REPORT_DIR=/data/reports`.

Example for a fully headless config (empty `config.toml` is **not** enough — the TOML still needs all required sections; env vars only override values, they do not create the file):

```yaml
# docker-compose.yml override example
services:
  502drive:
    environment:
      - GDCLONE__TELEGRAM__BOT_TOKEN=123456789:ABCdefGhIJKlmNoPQRsTUVwxyZ
      - GDCLONE__TELEGRAM__OWNER_TELEGRAM_ID=987654321
      - GDCLONE__GOOGLE_OAUTH__CLIENT_ID=xxxx.apps.googleusercontent.com
      - GDCLONE__GOOGLE_OAUTH__CLIENT_SECRET=yyyy
```

A minimal complete `config/config.toml` can be `config.sample.toml` with the four credentials replaced (or left as `REPLACE_ME` if provided via env).

## 4. One-time Google OAuth on a headless box

`auth login` uses an OAuth **loopback redirect**: the daemon binds `127.0.0.1` on a port in the 51000–51100 range and waits for the browser redirect. On a headless VPS/NAS there is no local browser, so do one of:

**Option A — authenticate on a desktop, run in Docker.** Copy the `config/config.toml` to a desktop machine, run `502drive auth login` there (same `client_id`/`client_secret`), then bring the `~/.local/share/gdclone-bot/state.db` (encrypted refresh token) back into the container's `/data` volume. The refresh token is encrypted with the `master.key` in the config directory, so the key file and the DB must travel together.

**Option B — publish the loopback range and open the URL remotely.** Keep the compose port mapping (`51000-51010:51000-51010`), run `auth login` inside the container, copy the printed Google authorization URL into a browser anywhere, and make sure your firewall/host network routes that port back to the container's published port. This requires the redirect to actually reach the machine where the login listener runs; if the browser cannot reach `http://localhost:51000` on the *client* machine, this will not work and Option A is simpler.

```bash
docker compose run --rm 502drive --config /config/config.toml auth login
docker compose run --rm 502drive --config /config/config.toml doctor
docker compose up -d
```

The full Google Cloud setup (project, Drive API, consent screen, Desktop client) is in [oauth-setup.md](oauth-setup.md).

## 5. Healthcheck

The Dockerfile installs `curl` but declares **no HEALTHCHECK** and the container exposes no listening HTTP port — the daemon is purely outbound (Telegram long polling + Google API). To monitor liveness, rely on `docker compose logs -f`, `docker ps` restart state, or add your own healthcheck that verifies the process is alive, e.g.:

```yaml
healthcheck:
  test: ["CMD", "sh", "-c", "kill -0 1"]
  interval: 60s
  timeout: 5s
  retries: 3
```

## 6. Permissions and upgrades

- `/config` and `/data` are owned by UID 10001 (`appuser`). If you pre-create `./config` and `./data` on the host as root, fix ownership: `sudo chown -R 10001:10001 config data`.
- To upgrade: `git pull && docker compose build && docker compose up -d`. The SQLite schema is migrated automatically on startup.
- Back up by stopping the container and copying `./data` (DB, WAL) and `./config` (config + `master.key`).
