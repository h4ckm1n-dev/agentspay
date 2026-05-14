#!/usr/bin/env bash
# Show the demo wallet's pubkey and the faucet URLs the operator must visit.
set -euo pipefail

WALLET=${1:-/var/lib/docker/volumes/agentspay_wallet-data/_data/devnet-wallet.json}
if [ ! -f "$WALLET" ]; then
  echo "wallet file not found at $WALLET" >&2
  exit 1
fi

PUBKEY=$(python3 -c "
import json, base58, sys
bytes_=json.load(open('$WALLET'))
print(base58.b58encode(bytes(bytes_[32:])).decode())
")
echo "pubkey: $PUBKEY"
echo "fund SOL : https://faucet.solana.com (paste the pubkey)"
echo "fund USDC: https://faucet.circle.com (select Solana Devnet)"
