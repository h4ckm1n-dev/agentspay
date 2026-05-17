# AgentsPay Python SDK

**Status: deferred indefinitely** per Plan v3.1.

The Python SDK was scaffolded against the Plan-v2 REST API (`/v1/payment-requirements`,
`/v1/payments/authorize`, etc.) that was never built. Plan v3 pivoted to a local MCP
server and only the TypeScript SDK was rewritten (see
[`packages/sdk-js/`](../sdk-js/)).

This directory is kept for two reasons:

1. **Future:** if there is demand for a Python wrapper, it would follow the same
   subprocess-transport pattern as `@agentspay/sdk-js` — spawn the
   `agentspay-mcp` binary and proxy `tools/call` over stdio JSON-RPC.
2. **Reservation:** so nobody else takes the `agentspay` PyPI name in the meantime.

The current `agentspay/` Python module here is **not functional** — it imports from a
REST API that does not exist. Do not depend on it.

## Recommended path today

If you are writing a Python program that needs the AgentsPay wallet:

- **Best:** run the `agentspay-mcp` binary directly as a subprocess and exchange
  MCP 2025-06-18 stdio JSON-RPC. See `services/web-shim/src/subprocess.rs` for
  the canonical implementation (Rust, but ports cleanly to Python's
  `asyncio.subprocess`).
- **Easier:** shell out to the `@agentspay/cli` tool with `--json` output and
  parse the result.

Both routes go through the same binary, see the same 5 tools, and inherit the
same security model (per-call + daily caps, SSRF guard, asset + decimals
validators).

## Reopening this SDK

Reopen when there is demand evidence — a concrete user asking for a Python entry
point. Until then, this stays a placeholder.

See [Plan.md §15](../../Plan.md) for the scope decision.
