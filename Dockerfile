# Stage 1: Build
FROM rust:1.83-bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
COPY migrations/ migrations/

RUN cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/server /usr/local/bin/server
COPY --from=builder /app/target/release/worker /usr/local/bin/worker
COPY migrations/ /app/migrations/

WORKDIR /app

EXPOSE 3000

CMD ["server"]
