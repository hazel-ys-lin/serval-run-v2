# Stage 1: Build
FROM rust:bookworm AS builder

WORKDIR /app

# Cache dependencies separately from source code.
# As long as Cargo.lock is unchanged, this layer is reused on rebuild.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs \
    && cargo build --release \
    && rm -rf src

# Copy real source and recompile only changed code
COPY src/ src/
COPY migrations/ migrations/
RUN touch src/main.rs && cargo build --release

# Stage 2: Runtime
FROM gcr.io/distroless/cc-debian12:nonroot

COPY --from=builder /app/target/release/server /usr/local/bin/server
COPY --from=builder /app/target/release/worker /usr/local/bin/worker
COPY --from=builder /app/migrations/ /app/migrations/

WORKDIR /app

EXPOSE 3000

CMD ["server"]
