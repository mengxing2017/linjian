.PHONY: install check check-web check-backend check-miniapp test-unit test-integration fmt api-dev dev-embedded web-dev miniapp-dev compose-config
install:
	pnpm --dir api-contract install --frozen-lockfile
	pnpm --dir frontend install --frozen-lockfile
check: check-web check-backend
check-web:
	pnpm --dir api-contract build
	pnpm --dir api-contract test
	pnpm --dir frontend typecheck
	pnpm --dir frontend lint
	pnpm --dir frontend test
	pnpm --dir frontend build
check-miniapp:
	pnpm --dir frontend --filter @linjian/miniapp typecheck
	pnpm --dir frontend --filter @linjian/miniapp lint
	pnpm --dir frontend --filter @linjian/miniapp test
	pnpm --dir frontend build:miniapp
check-backend:
	cargo fmt --manifest-path backend/Cargo.toml --all --check
	cargo clippy --locked --manifest-path backend/Cargo.toml --all-targets -- -D warnings
	cargo test --locked --manifest-path backend/Cargo.toml
fmt:
	cargo fmt --manifest-path backend/Cargo.toml --all
test-unit:
	cargo test --locked --manifest-path backend/Cargo.toml
test-integration:
	@test -n "$$LINJIAN_TEST_DATABASE_URL" || (echo 'Set isolated LINJIAN_TEST_DATABASE_URL' >&2; exit 1)
	cargo test --locked --manifest-path backend/Cargo.toml --test forum_database -- --ignored
api-dev:
	cargo run --locked --manifest-path backend/Cargo.toml --bin linjian-api
dev-embedded:
	cargo run --locked --manifest-path backend/Cargo.toml --example embedded_postgres
web-dev:
	pnpm --dir frontend dev
miniapp-dev:
	pnpm --dir frontend dev:miniapp
compose-config:
	docker compose --env-file .env config --quiet
