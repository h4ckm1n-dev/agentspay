# AgentsPay Docker stack

5 services: caddy (TLS reverse proxy) · web (Next.js) · shim (Rust HTTP bridge that spawns agentspay-mcp) · paid-endpoint (x402 demo provider) · redis (sessions + rate limits).

## First-time setup (production)

```bash
cd docker
cp .env.example .env
# edit DOMAIN and ACME_EMAIL

# Generate a fresh devnet wallet inside the volume:
docker volume create agentspay_wallet-data
# Then either generate via solana-keygen (if installed locally) or copy your
# existing ~/.agentspay/keypair.json into the volume:
docker run --rm -v agentspay_wallet-data:/data -v $HOME/.agentspay:/host alpine \
  sh -c "cp /host/keypair.json /data/devnet-wallet.json && cp /host/provider-keypair.json /data/provider-keypair.json"

# Fund it (manual — Circle faucet has a captcha)
./scripts/refill-wallet.sh

# Bring everything up
docker compose -f docker-compose.yml up -d --build

# Tail logs
docker compose logs -f
```

## Local dev (no TLS, no domain)

```bash
cd docker
docker compose -f docker-compose.yml -f docker-compose.local.yml up --build
```
Then open <http://localhost:3000>.

## Useful commands

- check wallet status: `curl https://$DOMAIN/api/devnet/wallet-status`
- restart shim only: `docker compose restart shim`
- backup wallet: `./scripts/backup-wallet.sh`
- view recent settled txs: any visitor's "Trigger" call updates `/api/stats/latest-tx` cache (persisted to SQLite at `AGENTSPAY_DEVNET_LEDGER_PATH`, survives shim restart).

## Build expectations

First Rust builder stage takes 5-10 minutes on a fresh cache (downloads + compiles solana-sdk, spl-token, axum, tonic, sea-orm). Subsequent builds are <30s thanks to BuildKit `--mount=type=cache`. The Next.js build is ~30-60s.
