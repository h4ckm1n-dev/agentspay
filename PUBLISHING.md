# Publishing AgentsPay to the MCP Registry

The official MCP registry (`registry.modelcontextprotocol.io`) hosts **metadata
only** — you publish the actual binary to crates.io first, then submit
`server.json`. AgentsPay lists via the **cargo** package type, so
`cargo install agentspay-mcp` is the install path (no npm wrapper needed to get
listed). A prebuilt-binary npm wrapper for instant `npx` UX is a planned
fast-follow.

Prereqs, in order. Steps 1–3 are done in-repo (see below); 4–6 need your
credentials and an interactive login.

## 1. License — done

Set to **Apache-2.0** (`license = "Apache-2.0"` in the workspace, `LICENSE` file
at repo root). Satisfies crates.io's SPDX requirement.

## 2. Version

Workspace version is bumped `0.1.0 → 0.3.0` to match the shipped product and the
crate's own user-agent string.

## 3. Crate metadata (`services/mcp/Cargo.toml`)

crates.io needs `description`, `repository`, `readme`, `license`, and benefits
from `keywords` + `categories`. The crate `README.md` carries the registry
ownership marker:

```
mcp-name: io.github.h4ckm1n-dev/agentspay
```

The registry fetches the crate's rendered README from crates.io and checks for
this exact line — do not remove it.

## 4. Publish the crate to crates.io

```bash
cargo login            # paste your crates.io API token (crates.io/settings/tokens)
cargo publish -p agentspay-mcp
```

`agentspay-mcp` depends only on crates.io crates (no sibling path deps), so it
publishes standalone. Verify at <https://crates.io/crates/agentspay-mcp>.

## 5. Install the registry publisher CLI

```bash
brew install mcp-publisher
# or: curl -L "https://github.com/modelcontextprotocol/registry/releases/latest/download/mcp-publisher_$(uname -s | tr '[:upper:]' '[:lower:]')_$(uname -m | sed 's/x86_64/amd64/;s/aarch64/arm64/').tar.gz" | tar xz mcp-publisher && sudo mv mcp-publisher /usr/local/bin/
```

## 6. Authenticate and publish the listing

GitHub OAuth grants the `io.github.h4ckm1n-dev/*` namespace, which matches the
name in `server.json`.

```bash
mcp-publisher login github     # device-code flow; must be the h4ckm1n-dev account
mcp-publisher publish          # reads ./server.json
```

Verify:

```bash
curl "https://registry.modelcontextprotocol.io/v0.1/servers?search=io.github.h4ckm1n-dev/agentspay"
```

## Secondary registries (metadata PRs / listings)

Once on crates.io + the official registry, cross-list where MCP discovery
happens. These are separate submissions, mostly a name + repo + install command:

- **Smithery** — <https://smithery.ai> (connect the GitHub repo)
- **mcp.so** — <https://mcp.so/submit>
- **PulseMCP** — <https://www.pulsemcp.com/submit>
- **Glama** — <https://glama.ai/mcp/servers> (auto-indexes public GitHub repos)
- **awesome-x402** — PR adding AgentsPay under the buy-side / agent-wallet section

Positioning line to reuse across all of them: *"A budget-controlled USDC wallet
for agents — keys never leave your machine: no account, no email, no OTP, no
enclave."*
