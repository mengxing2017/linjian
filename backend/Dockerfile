# syntax=docker/dockerfile:1
FROM rust:1.97.1-bookworm AS builder
RUN apt-get update && apt-get install -y --no-install-recommends cmake pkg-config ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /workspace/backend
COPY backend/Cargo.toml backend/Cargo.lock ./
COPY backend/src ./src
COPY backend/migrations ./migrations
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/workspace/backend/target \
    cargo build --locked --release --bins && cp target/release/linjian-api target/release/forum-snapshot /tmp/
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/* && useradd --system --uid 10001 --no-create-home linjian
COPY --from=builder /tmp/linjian-api /tmp/forum-snapshot /usr/local/bin/
USER 10001:10001
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/linjian-api"]
