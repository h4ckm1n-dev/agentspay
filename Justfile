set dotenv-load := true

default:
    just --list

dev:
    docker compose up

backend-check:
    cargo check --workspace

backend-test:
    cargo test --workspace

frontend-dev:
    pnpm --filter frontend dev

frontend-check:
    pnpm --filter frontend typecheck

sdk-check:
    pnpm --filter @agentspay/sdk typecheck

cli:
    pnpm --filter @agentspay/cli agentspay
