FROM rust:1.93-slim AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY sisi-infra ./sisi-infra
COPY src-tauri ./src-tauri

RUN cargo build --release -p sisi-infra

FROM debian:trixie-slim

RUN apt-get update && apt-get install -y \
    ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/sisi-infra .

EXPOSE 6640

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:6640/health || exit 1

CMD ["./sisi-infra"]
