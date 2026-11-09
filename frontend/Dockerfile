# syntax=docker/dockerfile:1
FROM node:22-bookworm-slim AS builder
WORKDIR /workspace/frontend
RUN corepack enable
COPY frontend/package.json frontend/pnpm-lock.yaml frontend/pnpm-workspace.yaml frontend/.npmrc ./
COPY frontend/apps/web/package.json apps/web/package.json
COPY frontend/apps/miniapp/package.json apps/miniapp/package.json
COPY api-contract ../api-contract
RUN pnpm --dir ../api-contract install --frozen-lockfile && pnpm --dir ../api-contract build && pnpm --filter @linjian/web install --frozen-lockfile
COPY frontend/apps/web apps/web
COPY frontend/tools/api-contract-aliases.ts tools/api-contract-aliases.ts
RUN pnpm build:web
FROM caddy:2.10-alpine
COPY frontend/Caddyfile /etc/caddy/Caddyfile
COPY --from=builder /workspace/frontend/apps/web/dist /srv
EXPOSE 80 443
