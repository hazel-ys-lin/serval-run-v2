# Stage 1: Build
FROM rust:bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
COPY migrations/ migrations/

RUN cargo build --release

# Stage 2: Runtime
FROM gcr.io/distroless/cc-debian12

COPY --from=builder /app/target/release/server /usr/local/bin/server
COPY --from=builder /app/target/release/worker /usr/local/bin/worker
COPY migrations/ /app/migrations/

WORKDIR /app

EXPOSE 3000

CMD ["server"]
