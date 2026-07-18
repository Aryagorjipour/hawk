# syntax=docker/dockerfile:1

FROM rust:1-bookworm AS builder
WORKDIR /app

# Pure rustls TLS — no system OpenSSL headers required for the build.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Cache dependency crates (dummy sources first)
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src \
    && printf 'fn main() {}\n' > src/main.rs \
    && printf 'pub fn _dummy() {}\n' > src/lib.rs \
    && cargo build --release 2>/dev/null || cargo build --release \
    && rm -rf src

COPY migrations ./migrations
COPY locales ./locales
COPY src ./src

RUN cargo build --release \
    && strip target/release/smart-hawk

FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 --shell /usr/sbin/nologin hawk \
    && mkdir -p /data \
    && chown hawk:hawk /data

COPY --from=builder /app/target/release/smart-hawk /usr/local/bin/smart-hawk

USER hawk
ENV RUST_LOG=info,smart_hawk=debug \
    DATABASE_URL=sqlite:/data/smart-hawk.db?mode=rwc \
    HOME=/home/hawk

VOLUME ["/data"]

# No HTTP server — process health is "container running"
STOPSIGNAL SIGTERM
CMD ["smart-hawk"]
