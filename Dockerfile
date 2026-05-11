# Stage 1: Build
FROM rust:bookworm AS builder

WORKDIR /app

# Note on caching: the classic "dummy main.rs" dependency-cache trick is
# fragile with workspace layouts (lib + multiple bins) because cargo's
# fingerprint cache happily reuses the empty stub lib's metadata even after
# the real source is copied in. We accept the slower build here. When build
# time becomes a real bottleneck, switch to cargo-chef rather than hand-
# rolling the dummy-build trick.
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
COPY migrations/ migrations/

RUN cargo build --release

# Stage 2: Runtime
FROM gcr.io/distroless/cc-debian12:nonroot

COPY --from=builder /app/target/release/server /usr/local/bin/server
COPY --from=builder /app/target/release/worker /usr/local/bin/worker
COPY --from=builder /app/migrations/ /app/migrations/

WORKDIR /app

EXPOSE 3000

CMD ["server"]
