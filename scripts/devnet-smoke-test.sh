#!/usr/bin/env bash
# Devnet smoke test for AgentsPay v0.3.
#
# Drives the local `agentspay-mcp` binary over stdio JSON-RPC, exercises the
# `agentspay_topup_info` tool to recover the agent pubkey, then attempts a
# real `agentspay_pay_url` call against the demo paid endpoint's
# `/real-quote/AAPL` route. Prints the Solscan URL for the resulting
# transaction (if settlement succeeds).
#
# This script cannot automate faucet funding — Circle's faucet requires a
# manual web captcha. The first run will report the pubkey so you can fund
# it; subsequent runs will exercise the full settlement path.
#
# Assumes:
#   - the demo paid endpoint is already running on http://localhost:3001
#     (e.g. `cargo run -p agentspay-paid-endpoint-demo` in another terminal).
#   - both binaries have been built: `cargo build -p agentspay-mcp
#     -p agentspay-paid-endpoint-demo`.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
MCP_BIN="${MCP_BIN:-$ROOT_DIR/target/debug/agentspay-mcp}"
ENDPOINT_URL="${ENDPOINT_URL:-http://localhost:3001/real-quote/AAPL}"
MAX_AMOUNT="${MAX_AMOUNT:-0.50}"
NETWORK="${AGENTSPAY_NETWORK:-solana-devnet}"

if [[ ! -x "$MCP_BIN" ]]; then
  echo "error: $MCP_BIN not found or not executable; run cargo build first" >&2
  exit 2
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "error: jq is required (brew install jq)" >&2
  exit 2
fi

INIT='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"smoke","version":"0.0"}}}'
INITED='{"jsonrpc":"2.0","method":"notifications/initialized"}'

topup_request() {
  cat <<EOF
$INIT
$INITED
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"agentspay_topup_info","arguments":{}}}
EOF
}

pay_request() {
  cat <<EOF
$INIT
$INITED
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"agentspay_set_budget","arguments":{"daily_usd":10.0,"per_call_usd":1.0}}}
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"agentspay_pay_url","arguments":{"url":"$ENDPOINT_URL","max_amount_usdc":"$MAX_AMOUNT"}}}
EOF
}

echo "== Step 1: agentspay_topup_info"
TOPUP_RESPONSE="$(
  (topup_request; sleep 1) \
    | AGENTSPAY_NETWORK="$NETWORK" "$MCP_BIN" 2>/dev/null \
    | grep -E '"id":2' \
    || true
)"
if [[ -z "$TOPUP_RESPONSE" ]]; then
  echo "error: empty MCP response on topup_info; agentspay-mcp may have crashed" >&2
  exit 3
fi

PUBKEY="$(echo "$TOPUP_RESPONSE" | jq -r '.result.content[0].text | fromjson | .pubkey')"
FAUCET_URL="$(echo "$TOPUP_RESPONSE" | jq -r '.result.content[0].text | fromjson | .faucet_url')"
echo "   pubkey:     $PUBKEY"
echo "   faucet_url: $FAUCET_URL"
echo "   (fund the pubkey with devnet USDC + a little SOL before continuing)"

echo
echo "== Step 2: agentspay_pay_url against $ENDPOINT_URL"
PAY_RESPONSE="$(
  (pay_request; sleep 45) \
    | AGENTSPAY_NETWORK="$NETWORK" "$MCP_BIN" 2>/dev/null \
    | grep -E '"id":3' \
    || true
)"

if [[ -z "$PAY_RESPONSE" ]]; then
  echo "error: empty MCP response on pay_url; agentspay-mcp may have crashed" >&2
  exit 4
fi

ERROR_MSG="$(echo "$PAY_RESPONSE" | jq -r '.error.message // empty')"
if [[ -n "$ERROR_MSG" ]]; then
  echo "   FAILED: $ERROR_MSG"
  echo "   This is usually because the pubkey has no devnet USDC yet."
  echo "   Fund $PUBKEY via $FAUCET_URL and try again."
  exit 1
fi

SIG="$(echo "$PAY_RESPONSE" | jq -r '.result.content[0].text | fromjson | .transaction')"
NETWORK_OUT="$(echo "$PAY_RESPONSE" | jq -r '.result.content[0].text | fromjson | .network')"
EXPLORER="$(echo "$PAY_RESPONSE" | jq -r '.result.content[0].text | fromjson | .explorer_url')"
AMOUNT="$(echo "$PAY_RESPONSE" | jq -r '.result.content[0].text | fromjson | .amount_charged_usdc')"
PAYMENT_STATUS="$(echo "$PAY_RESPONSE" | jq -r '.result.content[0].text | fromjson | .payment_status')"

echo "   OK"
echo "   payment_status: $PAYMENT_STATUS"
echo "   amount_charged: $AMOUNT USDC"
echo "   network:        $NETWORK_OUT"
echo "   transaction:    $SIG"
if [[ -n "$EXPLORER" && "$EXPLORER" != "null" ]]; then
  echo "   solscan:        $EXPLORER"
fi
