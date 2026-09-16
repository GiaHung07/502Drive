# Multi-stage build for 502Drive
FROM rust:bookworm AS builder

WORKDIR /app

# Copy dependency manifests, migrations, and source code
COPY Cargo.toml Cargo.lock ./
COPY migrations ./migrations
COPY src ./src

# Build production release binary
RUN cargo build --release --bin 502drive

# ------------------------------------------------------------------------------
# Minimal Runtime Stage
# ------------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

# Install CA certificates and timezone data for HTTPS and local time handling
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    tzdata \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security hardening
RUN groupadd -g 10001 appuser && \
    useradd -u 10001 -g appuser -s /bin/sh -m appuser

# Copy compiled binary from builder
COPY --from=builder /app/target/release/502drive /usr/local/bin/502drive

# Setup directories for configuration and persistent state
RUN mkdir -p /config /data && \
    chown -R appuser:appuser /config /data

USER appuser
WORKDIR /data

VOLUME ["/config", "/data"]

ENV RUST_LOG=info
ENV TZ=Asia/Ho_Chi_Minh

ENTRYPOINT ["/usr/local/bin/502drive"]
CMD ["--config", "/config/config.toml", "run"]
