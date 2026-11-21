# syntax=docker/dockerfile:1
FROM rust:1.97.1-bookworm AS builder
RUN if [ -f /etc/apt/sources.list ]; then \
      sed -i 's/deb.debian.org/mirrors.ustc.edu.cn/g' /etc/apt/sources.list; \
    fi \
    && if [ -f /etc/apt/sources.list.d/debian.sources ]; then \
      sed -i 's/deb.debian.org/mirrors.ustc.edu.cn/g' /etc/apt/sources.list.d/debian.sources; \
    fi \
    && apt-get update \
    && apt-get install -y --no-install-recommends cmake pkg-config ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /workspace/backend
COPY backend/Cargo.toml backend/Cargo.lock ./
COPY backend/src ./src
COPY backend/assets ./assets
COPY backend/migrations ./migrations
COPY api-contract/fixtures/school-permission-catalog.json ../api-contract/fixtures/school-permission-catalog.json
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/workspace/backend/target \
    cargo build --locked --release --bins && cp target/release/linjian-api target/release/forum-snapshot /tmp/
FROM debian:bookworm-slim
RUN if [ -f /etc/apt/sources.list ]; then \
      sed -i 's/deb.debian.org/mirrors.ustc.edu.cn/g' /etc/apt/sources.list; \
    fi \
    && if [ -f /etc/apt/sources.list.d/debian.sources ]; then \
      sed -i 's/deb.debian.org/mirrors.ustc.edu.cn/g' /etc/apt/sources.list.d/debian.sources; \
    fi \
    && apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --no-create-home linjian
COPY --from=builder /tmp/linjian-api /tmp/forum-snapshot /usr/local/bin/
ENV LINJIAN_MEDIA_ROOT=/data/media
RUN mkdir -p /data/media && chown 10001:10001 /data/media
USER 10001:10001
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/linjian-api"]
