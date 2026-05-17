# @agentspay/cli

Command-line tool for [AgentsPay](https://agentspay.dev). Wraps the `agentspay-mcp` binary and exposes the five MCP tools as terminal commands.

## Install

```bash
pnpm add -g @agentspay/cli
# or: npm install -g @agentspay/cli
```

You also need the `agentspay-mcp` binary on PATH. Build it from the repo:

```bash
cargo build --release -p agentspay-mcp
cp target/release/agentspay-mcp /usr/local/bin/
```

Or pass `--bin <path>` on every call, or set `AGENTSPAY_MCP_BIN` env var.

## Usage

```bash
agentspay --help
agentspay --version

# Read the wallet
agentspay balance

# Set spending caps
agentspay set-budget --daily 25 --per-call 1

# Pay an x402 URL
agentspay pay-url https://api.example.com/quote --max 0.50

# Recent activity (including rejected attempts)
agentspay audit-log --limit 10

# Pubkey + faucet URLs for funding
agentspay topup-info
```

## Global flags

| Flag | Default | Purpose |
|---|---|---|
| `--network <name>` | `solana-devnet` | `sandbox` / `solana-devnet` / `solana-mainnet` |
| `--bin <path>` | PATH lookup | Override `agentspay-mcp` binary location |
| `--keypair <path>` | binary default | Override `AGENTSPAY_KEYPAIR_PATH` |
| `--json` | off | Raw JSON output for piping/scripting |
| `--debug` | off | Inherit subprocess stderr (the pretty banner) |
| `-h, --help` | | Show help |
| `-v, --version` | | Show version |

## Exit codes

- `0` — success
- `2` — argv parsing failure (missing required flag, unknown command)
- `1` — anything else (binary not found, transport timeout, cap exceeded, x402 settlement failure, invalid input from the SDK)

All errors print a one-line message on stderr, prefixed with the category in red (e.g. `daily budget exceeded:`, `per-call cap exceeded:`, `settlement failed:`). The category name comes from the SDK error class; pipe stderr through `grep` if you need to alert on a specific category.

## Examples

```bash
# Quick balance check, piped to jq
agentspay balance --json | jq .available_usdc

# Pay the demo provider on a local devnet (requires AGENTSPAY_ALLOW_PRIVATE_HOSTS=1)
AGENTSPAY_ALLOW_PRIVATE_HOSTS=1 \
  agentspay pay-url http://localhost:3001/real-quote/AAPL --max 0.50

# Set conservative caps, then audit recent activity
agentspay set-budget --daily 5 --per-call 0.25
agentspay audit-log --limit 5
```

## Why a CLI when there's an SDK?

The SDK is for programs that use AgentsPay as a library. The CLI is for everything else:

- Shell scripts that need to pay an endpoint
- Quick balance checks without writing TypeScript
- CI smoke tests
- Manual debugging of MCP tool behavior

Both spawn the same `agentspay-mcp` binary under the hood, so they share the security model, the audit log, and the budget caps.

## License

MIT
