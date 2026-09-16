# Google OAuth Setup (BYOK)

502Drive uses **BYOK — Bring Your Own Keys**: you create your own Google Cloud OAuth client and grant the bot access to *your own* Drive, the same model as rclone. Nothing in this setup shares credentials with the 502Drive project (there are no project servers; see [PRIVACY.md](../PRIVACY.md)).

Time required: about 10 minutes. Applies to all platforms ([Linux](install-linux.md), [Windows](install-windows.md), [Docker](install-docker.md)).

## 1. Create a Google Cloud project

1. Go to <https://console.cloud.google.com/> and sign in.
2. Click the project dropdown → **New Project** (any name, e.g. `502drive-personal`).

## 2. Enable the Drive API

1. Open **APIs & Services → Library**.
2. Search for **Google Drive API** and click **Enable**.

## 3. Configure the OAuth consent screen

1. Go to **APIs & Services → OAuth consent screen**.
2. Choose **External** as the user type (even for personal use) and click **Create**.
3. Fill in the required fields: app name, your email as user support email, and developer contact email. No scopes need to be added manually here; the Drive scope is requested at login.
4. Under **Audience/Publishing status**, click **Publish App**. Do not submit the app for Google verification — you do not need it for personal use (see the BYOK section below).

> Skipping "Publish App" is the most common setup error: the login then fails with `Error 403: access_denied` because Google only allows registered test users. See [troubleshooting.md](troubleshooting.md).

## 4. Create an OAuth Desktop client

1. Go to **APIs & Services → Credentials → Create credentials → OAuth client ID**.
2. Application type: **Desktop app** (this matters — the daemon uses a loopback redirect, not a web redirect).
3. Click **Create** and copy the **Client ID** (`....apps.googleusercontent.com`) and **Client secret**.

No redirect URIs need to be configured; desktop clients use `http://localhost:51000-51100` loopback redirects out of the box.

## 5. Put credentials in the config

Desktop (`~/.config/gdclone-bot/config.toml`) or Docker (`/config/config.toml`):

```toml
[google_oauth]
client_id = "YOUR_CLIENT_ID.apps.googleusercontent.com"
client_secret = "YOUR_CLIENT_SECRET"
redirect_port_start = 51000
redirect_port_end = 51100
scope = "https://www.googleapis.com/auth/drive"
```

Alternatively set `GDCLONE__GOOGLE_OAUTH__CLIENT_ID` and `GDCLONE__GOOGLE_OAUTH__CLIENT_SECRET` environment variables (full table in [install-docker.md](install-docker.md)).

## 6. Log in

```bash
502drive auth login
```

What happens:

1. The daemon picks a free port in the 51000–51100 range and binds it to `127.0.0.1` only (loopback PKCE — no public ports).
2. It opens your default browser at the Google authorization URL. On a headless machine it prints the URL instead — copy it into any browser (see [install-docker.md](install-docker.md) §4 for the headless options).
3. Sign in with the Google account that owns the Drive you want to sync/clone, approve the warning ("Google hasn't verified this app" → **Continue**), and grant the Drive scope.
4. Google redirects to `http://localhost:<port>`; the daemon exchanges the code (PKCE), validates the credentials, and stores the **AES-256-GCM-encrypted** refresh token in the local SQLite DB. The encryption key is a local `master.key` file (created with `0600` permissions) in the config directory.

`telegram /connect` does not complete OAuth — it only points you to this CLI flow.

## 7. Verify

```bash
502drive doctor
```

Expected output includes `google_account: OK - connected`, `google_token_refresh: OK`, and `drive.about: <your email>`. Then start the daemon (`502drive run` / your service) and finish Telegram setup — see [README](../README.md#configuration-configtoml).

Revoking access later: `502drive auth revoke` (or remove the app under [Google account → Security → Third-party access](https://myaccount.google.com/permissions)).

---

## Why self-provided credentials (BYOK)?

Google's OAuth rules for unverified apps that use sensitive scopes (Drive is one) impose a **100-user cap on authorization** and a seven-day token lifetime for apps in "Testing" status. If 502Drive shipped its own client ID:

- every user would count against that shared cap;
- a verified public app would require Google to audit the project and its broad `drive` scope — impractical for a personal, open-source tool;
- the project would have to hold a client secret that pairs with a publicly distributed binary (secrets in public repos are extracted within hours).

By providing your own client ID/secret:

- the "app" is yours, the only authorized user is you, and the 100-user cap is irrelevant;
- your refresh token is only ever exchanged with Google directly from your machine — no third party (including this project) is in the token path;
- you can revoke everything from the Google Cloud console or your Google account page at any time.

This mirrors the rclone model: the tool ships code, you ship credentials.
