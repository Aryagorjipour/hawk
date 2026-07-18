# syntax=docker/dockerfile:1

FROM rust:1-bookworm AS builder
WORKDIR /app

# Pure rustls — no system OpenSSL headers required.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates binutils \
    && rm -rf /var/lib/apt/lists/*

# --- dependency cache layer (dummy crate sources only) ---
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src \
    && printf 'fn main() { panic!("docker dep-cache dummy — rebuild failed"); }\n' > src/main.rs \
    && printf 'pub fn _dummy() {}\n' > src/lib.rs \
    && cargo build --release \
    && rm -rf src \
    && rm -f target/release/smart-hawk \
    && rm -rf target/release/deps/smart_hawk-* \
    && rm -f target/release/.fingerprint/smart-hawk-* 2>/dev/null || true

# --- real sources ---
COPY migrations ./migrations
COPY locales ./locales
COPY src ./src

# Critical: Docker COPY can preserve mtimes older than the dummy build artifacts,
# so Cargo may skip rebuilding and ship the empty dummy binary (exit 0, no logs).
# Force a full recompile of this package and verify the real boot string is present.
RUN find src migrations locales -type f -exec touch -d "now" {} + \
    && rm -f target/release/smart-hawk \
    && rm -rf target/release/deps/smart_hawk-* \
    && rm -rf target/release/.fingerprint/smart-hawk-* \
    && cargo build --release \
    && strip target/release/smart-hawk \
    && strings target/release/smart-hawk | grep -q "smart-hawk: starting" \
    || (echo "ERROR: release binary is not the real smart-hawk (missing boot string)" && exit 1)

FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 --shell /usr/sbin/nologin hawk \
    && mkdir -p /data \
    && chown hawk:hawk /data

COPY --from=builder /app/target/release/smart-hawk /usr/local/bin/smart-hawk

USER hawk
ENV RUST_LOG=info,smart_hawk=debug,teloxide=info \
    DATABASE_URL=sqlite:/data/smart-hawk.db?mode=rwc \
    HOME=/home/hawk \
    RUST_BACKTRACE=1

VOLUME ["/data"]
STOPSIGNAL SIGTERM
ENTRYPOINT ["smart-hawk"]
