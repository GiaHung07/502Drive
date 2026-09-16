# Multi-stage build for the 502Drive headless daemon (Telegram bot + sync engine)
# The Tauri desktop GUI is NOT built here — use `cargo tauri build` on a desktop OS.
FROM rust:bookworm AS builder

WORKDIR /app

# The root crate is a Cargo workspace whose members include src-tauri, so the
# member manifests must be present even when only the `502drive` bin is built.
COPY Cargo.toml Cargo.lock ./
COPY migrations ./migrations
COPY src ./src
COPY src-tauri ./src-tauri

RUN cargo build --release --bin 502drive

# ------------------------------------------------------------------------------
# Minimal Runtime Stage
# ------------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

# CA certificates for HTTPS, tzdata for local time, curl for manual probes
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    tzdata \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Non-root user for security hardening
RUN groupadd -g 10001 appuser && \
    useradd -u 10001 -g appuser -s /bin/sh -m appuser

COPY --from=builder /app/target/release/502drive /usr/local/bin/502drive
COPY --from=builder /app/target/release/gdclone-bot /usr/local/bin/gdclone-bot

RUN mkdir -p /config /data && \
    chown -R appuser:appuser /config /data

USER appuser
WORKDIR /data

VOLUME ["/config", "/data"]

ENV RUST_LOG=info
ENV TZ=Asia/Ho_Chi_Minh
# Keep all mutable state on the mounted /data volume (defaults would land in
# the container-internal home and vanish on recreate).
ENV GDCLONE__STORAGE__DB_PATH=/data/state.db
ENV GDCLONE__STORAGE__LOG_DIR=/data/logs
ENV GDCLONE__STORAGE__REPORT_DIR=/data/reports

# `status` opens the SQLite database read-only — a cheap liveness probe that
# also fails loudly when /config/config.toml is missing or malformed.
HEALTHCHECK --interval=60s --timeout=15s --start-period=30s --retries=3 \
    CMD ["502drive", "--config", "/config/config.toml", "status"]

ENTRYPOINT ["/usr/local/bin/502drive"]
CMD ["--config", "/config/config.toml", "run"]
