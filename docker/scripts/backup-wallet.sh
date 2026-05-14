#!/usr/bin/env bash
set -euo pipefail
SRC=${1:-/var/lib/docker/volumes/agentspay_wallet-data/_data/devnet-wallet.json}
DEST=${2:-$HOME/devnet-wallet-$(date +%Y%m%d-%H%M%S).json}
cp "$SRC" "$DEST"
chmod 0600 "$DEST"
echo "backed up to $DEST"
