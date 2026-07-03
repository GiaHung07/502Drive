# Threat Model

Primary assets:

- Telegram bot token.
- Google OAuth refresh token.
- Local master key.
- SQLite DB containing mappings, reports, and operational history.
- Drive resource keys.

Primary controls:

- Long polling only; no public webhook listener.
- OAuth loopback binds `127.0.0.1` only.
- PKCE and state validation for OAuth.
- Refresh tokens are encrypted before DB storage with a master key outside the DB.
- Telegram authorization is checked before command dispatch.
- Callback state is server-side and owner-bound.
- SQL is parameterized.
- Logs redact tokens, auth codes, PKCE verifier, client secret, and resource keys.

Operational risks:

- A local user account compromise can access config and local data.
- Google broad Drive scope is powerful; this is documented as a local personal app tradeoff.
- Windows service context can make CurrentUser secrets inaccessible; the app must fail clearly rather than silently using a different account.

