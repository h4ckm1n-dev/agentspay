import type { Metadata } from "next";
import * as React from "react";
import { CodeBlock } from "@/components/ui/CodeBlock";
import { Footer } from "@/components/sections/Footer";
import {
  BreadcrumbStructuredData,
  TechArticleStructuredData,
} from "@/components/seo/PageStructuredData";

export const metadata: Metadata = {
  title: "Docs — Install, tools, payment flow, security",
  description:
    "Developer documentation for AgentsPay: install agentspay-mcp in Claude Code / Cursor / Cline / Zed, configure budgets, sign Solana devnet USDC settlements, audit every transaction. Five MCP tools, one binary, SQLite ledger.",
  alternates: { canonical: "/docs" },
  openGraph: {
    title: "AgentsPay docs — MCP wallet for AI agents",
    description:
      "Install, configure, and verify the AgentsPay MCP wallet. Tool reference, payment flow, budgets, devnet funding, security model.",
    url: "/docs",
    type: "article",
  },
  twitter: {
    card: "summary_large_image",
    title: "AgentsPay docs",
    description:
      "Tool reference and developer docs for the AgentsPay MCP wallet.",
  },
};

const TOC = [
  ["overview", "Overview"],
  ["install", "Install"],
  ["hosts", "MCP hosts"],
  ["first-run", "First run"],
  ["sdk-cli", "SDK and CLI"],
  ["tools", "Tools"],
  ["flow", "Payment flow"],
  ["policy", "Budgets and ledger"],
  ["devnet", "Devnet funding"],
  ["docker", "Website demo"],
  ["env", "Environment"],
  ["security", "Security model"],
  ["troubleshooting", "Troubleshooting"],
  ["commands", "Commands"],
] as const;

const TOOL_REFERENCE = [
  {
    name: "agentspay_balance",
    intent:
      "Inspect the local wallet row, budget settings, today's spend, and signing pubkey.",
    args: "No arguments.",
    returns: [
      "available_usdc: local wallet-row balance string",
      "today_spent_usdc: sum of paid ledger rows since UTC midnight",
      "daily_cap_usdc and per_call_cap_usdc: configured budget or displayed default",
      "budget_remaining_today_usdc: daily cap minus today's ledger spend",
      "environment: sandbox, solana-devnet, or solana-mainnet",
      "solana_pubkey: base58 signer pubkey",
    ],
    call: "agentspay_balance()",
    response: `{
  "available_usdc": "100.00",
  "today_spent_usdc": "0.10",
  "daily_cap_usdc": "25.00",
  "per_call_cap_usdc": "1.00",
  "budget_remaining_today_usdc": "24.90",
  "environment": "solana-devnet",
  "solana_pubkey": "GmBDzsdcPBNpeGchxX2GkZTKYtuCKnj7wyHiYaL9zPEm"
}`,
  },
  {
    name: "agentspay_pay_url",
    intent:
      "Call an x402-priced URL, cap the quoted amount, sign if allowed, retry, and persist proof.",
    args: `{
  "url": "http://localhost:3001/real-quote/AAPL",
  "max_amount_usdc": "0.50"
}`,
    returns: [
      "status and payment_id",
      "amount_charged_usdc quoted from the x402 requirement",
      "ledger_entry_id when a paid call is recorded",
      "transaction and explorer_url for Solana settlements",
      "payment_status: paid or none",
      "body: upstream response body as a string",
    ],
    call: "agentspay_pay_url(url, max_amount_usdc)",
    response: `{
  "status": "ok",
  "amount_charged_usdc": "0.10",
  "payment_status": "paid",
  "network": "solana-devnet",
  "transaction": "4pGRMVgu7j5itCs7Vf6G9FTQW2Q1B2SjCEKHszLjvF9eVagWvtWq8aJWuYz1JNpBQr4CsbYRXSb9aWAu5hv6jYau",
  "explorer_url": "https://solscan.io/tx/4pGR...jYau?cluster=devnet"
}`,
  },
  {
    name: "agentspay_set_budget",
    intent: "Create or update the active per-call and daily spend policy.",
    args: `{
  "daily_usd": 25,
  "per_call_usd": 1
}`,
    returns: [
      "agent_id: currently default",
      "daily_usd and per_call_usd as accepted",
      "updated_at_rfc3339 for auditability",
    ],
    call: "agentspay_set_budget(daily_usd, per_call_usd)",
    response: `{
  "agent_id": "default",
  "daily_usd": 25,
  "per_call_usd": 1,
  "updated_at_rfc3339": "2026-05-16T13:00:00Z"
}`,
  },
  {
    name: "agentspay_audit_log",
    intent:
      "Return recent tool attempts, including rejected and successful payment calls.",
    args: `{
  "limit": 20
}`,
    returns: [
      "entries ordered newest first",
      "total count in the local audit table",
      "returned count after the limit is applied",
      "limit defaults to 20 and clamps at 100",
    ],
    call: "agentspay_audit_log(limit?)",
    response: `{
  "returned": 2,
  "total": 2,
  "entries": [
    {
      "tool": "agentspay_pay_url",
      "amount_usdc": "0.10",
      "status": "ok payment_status=paid network=solana-devnet"
    }
  ]
}`,
  },
  {
    name: "agentspay_topup_info",
    intent:
      "Show the pubkey and faucet instructions needed to fund the local signer.",
    args: "No arguments.",
    returns: [
      "pubkey to paste into faucets",
      "network",
      "Circle USDC faucet URL",
      "Solana SOL faucet URL",
      "manual funding instructions",
    ],
    call: "agentspay_topup_info()",
    response: `{
  "pubkey": "GmBDzsdcPBNpeGchxX2GkZTKYtuCKnj7wyHiYaL9zPEm",
  "network": "solana-devnet",
  "faucet_url": "https://faucet.circle.com",
  "sol_faucet_url": "https://faucet.solana.com"
}`,
  },
] as const;

const ENV_VARS = [
  [
    "AGENTSPAY_NETWORK",
    "solana-devnet",
    "MCP settlement mode. Use sandbox for no-chain local testing.",
  ],
  [
    "AGENTSPAY_KEYPAIR_PATH",
    "~/.agentspay/keypair.json",
    "Solana CLI-compatible signer JSON, created mode 0600.",
  ],
  [
    "AGENTSPAY_DATABASE_URL",
    "sqlite://~/.agentspay/agentspay-mcp.db?mode=rwc",
    "SeaORM SQLite ledger and audit database.",
  ],
  [
    "AGENTSPAY_SOLANA_RPC_URL",
    "https://api.devnet.solana.com",
    "RPC endpoint used to fetch blockhashes and balances.",
  ],
  ["RUST_LOG", "agentspay_mcp=info", "Structured logs and warning verbosity."],
  ["NO_COLOR", "unset", "Disable stderr ANSI color from the MCP banner."],
  ["FORCE_COLOR", "unset", "Force ANSI color for Docker logs or CI output."],
  [
    "AGENTSPAY_SHIM_LISTEN_ADDR",
    "0.0.0.0:8080",
    "HTTP bind address for the web shim.",
  ],
  [
    "AGENTSPAY_MCP_BINARY",
    "/usr/local/bin/agentspay-mcp",
    "Binary spawned by the shim for browser demos.",
  ],
  [
    "AGENTSPAY_REDIS_URL",
    "unset",
    "Redis sessions and rate limits. Unset uses in-memory stores.",
  ],
  [
    "AGENTSPAY_DEVNET_WALLET_PATH",
    "/data/devnet-wallet.json",
    "Funded demo signer used by /api/devnet/trigger.",
  ],
  [
    "AGENTSPAY_DEVNET_LEDGER_PATH",
    "/data/devnet-ledger.db",
    "Persistent demo ledger and latest-tx cache.",
  ],
  [
    "AGENTSPAY_PAID_ENDPOINT_URL",
    "http://localhost:3001",
    "Demo x402 provider target.",
  ],
  [
    "AGENTSPAY_PROVIDER_KEYPAIR",
    "~/.agentspay/provider-keypair.json",
    "Provider receiver keypair for the demo endpoint.",
  ],
  [
    "AGENTSPAY_DEMO_PAYTO",
    "derived provider pubkey",
    "Explicit receiver pubkey override for demo quotes.",
  ],
  [
    "AGENTSPAY_USE_FACILITATOR",
    "false",
    "Opt-in x402.org facilitator path in the demo provider.",
  ],
  [
    "AGENTSPAY_FACILITATOR_URL",
    "https://x402.org/facilitator",
    "Facilitator base URL when enabled.",
  ],
  [
    "AGENTSPAY_ALLOW_PRIVATE_HOSTS",
    "unset (= disabled)",
    "When set to 1, allow pay_url to fetch loopback / RFC1918 / link-local hosts. Required for local dev against the demo provider; never set in production.",
  ],
  [
    "AGENTSPAY_ALLOWED_ORIGINS",
    "unset (= origin guard disabled)",
    "Comma-separated allowlist of browser origins for mutating shim endpoints. Production should set this to your deployed frontend origin.",
  ],
] as const;

const TROUBLE = [
  {
    symptom: "The agent can see tools, but payment is rejected by budget.",
    cause:
      "The quoted x402 amount is above max_amount_usdc, above per_call_usd, or would cross daily_usd.",
    fix: "Call agentspay_balance, then lower the URL price or raise caps with agentspay_set_budget.",
  },
  {
    symptom: "Devnet payment fails with account debit or insufficient funds.",
    cause:
      "The signer has no SOL for fees, no devnet USDC, or no initialized token account.",
    fix: "Run agentspay_topup_info and fund the returned pubkey from both Solana and Circle faucets.",
  },
  {
    symptom: "MCP host starts but cannot find AgentsPay.",
    cause:
      "The host config points at a relative path or a binary that was not built.",
    fix: "Use an absolute path to target/release/agentspay-mcp and run cargo build --release -p agentspay-mcp again.",
  },
  {
    symptom: "The website sandbox returns session gone.",
    cause:
      "Browser session state expired. The shim keeps sandbox sessions for 30 minutes.",
    fix: "Refresh the page or create a new sandbox session.",
  },
  {
    symptom: "The devnet demo button is disabled or drained.",
    cause: "The server-controlled demo wallet is below 0.05 SOL or 2 USDC.",
    fix: "Seed docker_wallet-data with devnet-wallet.json and refill it from the faucets.",
  },
  {
    symptom: "Facilitator mode rejects the Solana payload.",
    cause:
      "The direct-RPC path signs a fee-paying transaction. Some facilitators expect a sponsored format.",
    fix: "Use direct RPC for the current demo path. Treat facilitator mode as experimental until the payload format is refactored.",
  },
] as const;

export default function DocsPage() {
  return (
    <main>
      <BreadcrumbStructuredData
        trail={[
          { name: "Home", url: "/" },
          { name: "Docs", url: "/docs" },
        ]}
      />
      <TechArticleStructuredData
        headline="AgentsPay developer documentation"
        description="Install, configure, and verify the AgentsPay MCP wallet. Tool reference, payment flow, budgets, devnet funding, security model."
        path="/docs"
        sections={TOC.map(([, label]) => label)}
      />
      <section className="page-hero">
        <div className="mx-auto max-w-7xl">
          <div className="section-kicker">Developer docs</div>
          <h1 className="max-w-4xl text-4xl font-semibold leading-tight sm:text-6xl">
            Build agents that can pay, with a wallet they cannot drain.
          </h1>
          <p className="mt-4 max-w-3xl text-sm leading-relaxed text-fg-muted sm:text-base">
            AgentsPay is a local MCP server. It gives an agent five tools for
            balance, budget, x402 payment, audit logs, and top-up instructions.
            The current product is devnet-first, self-custodial, and designed
            for developers shipping inside Claude Code, Cursor, Cline, or Zed.
          </p>
        </div>
      </section>

      <div className="mx-auto grid max-w-7xl gap-10 px-5 py-10 sm:px-6 lg:grid-cols-[260px_1fr] lg:px-8">
        <aside className="hidden lg:block">
          <nav className="sticky top-24 space-y-1 rounded-lg border border-border bg-black/30 p-3 font-mono text-xs">
            {TOC.map(([href, label]) => (
              <a
                key={href}
                href={`#${href}`}
                className="block rounded px-2 py-1.5 text-fg-muted transition hover:bg-bg-elev hover:text-fg"
              >
                {label}
              </a>
            ))}
          </nav>
        </aside>

        <div className="min-w-0 space-y-16">
          <DocSection
            id="overview"
            eyebrow="01"
            title="What You Are Installing"
          >
            <div className="grid gap-4 md:grid-cols-3">
              <Fact
                title="Local binary"
                body="The MCP host launches agentspay-mcp over stdio. There is no hosted AgentsPay control plane in the v0.3 path."
              />
              <Fact
                title="Self-custodial signer"
                body="The keypair lives on your machine at ~/.agentspay/keypair.json unless AGENTSPAY_KEYPAIR_PATH overrides it."
              />
              <Fact
                title="Policy before signature"
                body="max_amount_usdc and configured budgets are checked before the code builds a Solana transaction."
              />
            </div>
            <Flow>
              <span>MCP host</span>
              <span>agentspay-mcp</span>
              <span>x402 endpoint</span>
              <span>Solana devnet</span>
              <span>SQLite audit log</span>
            </Flow>
          </DocSection>

          <DocSection id="install" eyebrow="02" title="Install In 60 Seconds">
            <p>
              Build the release binary and register it with your MCP host. Use
              an absolute path once you move beyond a local checkout.
            </p>
            <div className="mt-5 space-y-3">
              <CodeBlock value="git clone https://github.com/h4ckm1n-dev/agentspay && cd agentspay" />
              <CodeBlock value="cargo build --release -p agentspay-mcp" />
              <CodeBlock value="claude mcp add agentspay $PWD/target/release/agentspay-mcp" />
            </div>
            <Callout title="Important">
              Call <Mono>agentspay_set_budget</Mono> before the first paid call.
              The balance view displays a default budget when no row exists, but
              the active enforcement policy is created by
              <Mono> agentspay_set_budget</Mono>.
            </Callout>
          </DocSection>

          <DocSection id="hosts" eyebrow="03" title="MCP Host Configs">
            <p>
              Every host needs the same thing: a command that launches the
              binary. Environment variables are optional and can be added when
              you want sandbox mode, a custom ledger, or a custom keypair path.
            </p>
            <div className="grid gap-4 md:grid-cols-2">
              <Host
                title="Claude Code"
                code="claude mcp add agentspay /abs/path/agentspay-mcp"
              />
              <Host
                title="Cursor"
                code={`{
  "mcpServers": {
    "agentspay": {
      "command": "/abs/path/agentspay-mcp"
    }
  }
}`}
              />
              <Host
                title="Cline"
                code={`{
  "mcpServers": {
    "agentspay": {
      "command": "/abs/path/agentspay-mcp",
      "env": {
        "AGENTSPAY_NETWORK": "solana-devnet"
      }
    }
  }
}`}
              />
              <Host
                title="Zed"
                code={`{
  "context_servers": {
    "agentspay": {
      "command": "/abs/path/agentspay-mcp"
    }
  }
}`}
              />
            </div>
          </DocSection>

          <DocSection id="first-run" eyebrow="04" title="First Run Checklist">
            <StepList
              items={[
                "Start the MCP host and confirm it lists the five AgentsPay tools.",
                "Call agentspay_topup_info and copy the returned pubkey.",
                "Fund that pubkey with devnet SOL and devnet USDC.",
                "Call agentspay_set_budget with a small daily and per-call cap.",
                "Call agentspay_balance and verify the environment is solana-devnet.",
                "Try a sandbox endpoint first, then a devnet x402 endpoint.",
              ]}
            />
            <CodePanel
              title="Suggested first prompts"
              code={`Call agentspay_topup_info.
Set my AgentsPay budget to 5 USDC per day and 0.25 USDC per call.
Show my AgentsPay balance.
Pay http://localhost:3001/real-quote/AAPL with max_amount_usdc 0.25.`}
            />
          </DocSection>

          <DocSection id="sdk-cli" eyebrow="05" title="TypeScript SDK and CLI">
            <p className="mb-4 max-w-3xl text-sm leading-relaxed text-fg-muted">
              The MCP binary is the primary surface, but you don&apos;t need an
              MCP host to use AgentsPay. Two npm packages wrap the binary:
              <code className="mx-1 rounded bg-bg-elev px-1.5 py-0.5 font-mono text-xs">
                @agentspay/sdk-js
              </code>{" "}
              for Node.js apps, and
              <code className="mx-1 rounded bg-bg-elev px-1.5 py-0.5 font-mono text-xs">
                @agentspay/cli
              </code>{" "}
              for a terminal command.
            </p>
            <div className="grid gap-4 md:grid-cols-2">
              <CodePanel
                title="TypeScript SDK"
                code={`pnpm add @agentspay/sdk-js
# also requires agentspay-mcp on PATH

import { AgentsPayClient } from "@agentspay/sdk-js";
const client = new AgentsPayClient({ network: "solana-devnet" });
const balance = await client.balance();
const r = await client.payUrl({
  url: "https://api.example.com/quote",
  maxAmountUsdc: "0.50",
});`}
              />
              <CodePanel
                title="CLI"
                code={`pnpm add -g @agentspay/cli
agentspay --help

agentspay balance
agentspay pay-url <url> --max 0.50
agentspay set-budget --daily 25 --per-call 1
agentspay audit-log --limit 5
agentspay topup-info
# add --json for raw output`}
              />
            </div>
            <Callout title="Same binary under the hood">
              Both the SDK and CLI spawn the same{" "}
              <code className="font-mono text-fg">agentspay-mcp</code> binary as
              a subprocess and talk JSON-RPC over stdio. Whatever your MCP host
              sees, the SDK and CLI see the same. Typed errors
              (BudgetExceededError, PerCallCapExceededError,
              X402SettlementError, ...) let you handle each failure mode
              discretely.
            </Callout>
            <p className="mt-4 text-sm text-fg-muted">
              Full package documentation:{" "}
              <a
                href="https://github.com/h4ckm1n-dev/agentspay/blob/main/packages/sdk-js/README.md"
                className="text-accent underline-offset-4 hover:underline"
              >
                @agentspay/sdk-js README
              </a>{" "}
              ·{" "}
              <a
                href="https://github.com/h4ckm1n-dev/agentspay/blob/main/packages/cli/README.md"
                className="text-accent underline-offset-4 hover:underline"
              >
                @agentspay/cli README
              </a>
              . Both ship the same five tools with typed errors and pretty +
              JSON output.
            </p>
          </DocSection>

          <DocSection id="tools" eyebrow="06" title="Tool Reference">
            <div className="space-y-5">
              {TOOL_REFERENCE.map((tool) => (
                <ToolCard key={tool.name} tool={tool} />
              ))}
            </div>
          </DocSection>

          <DocSection id="flow" eyebrow="07" title="Payment Flow">
            <StepList
              items={[
                "agentspay_pay_url validates the URL and max_amount_usdc.",
                "It performs a GET probe against the target URL.",
                "If the target returns 200, AgentsPay records a no-payment audit entry and returns the body.",
                "If the target returns 402, AgentsPay parses accepts[] and selects the entry matching AGENTSPAY_NETWORK.",
                "The x402 quoted amount must be <= max_amount_usdc.",
                "If a budget row exists, per_call_usd and daily_usd are checked while a Tokio mutex serializes payment calls.",
                "Sandbox mode emits a placeholder X-Payment payload.",
                "Solana devnet mode builds an SPL transfer_checked transaction and base64 encodes it into X-Payment.",
                "AgentsPay retries the same URL with X-Payment and parses X-Payment-Response.",
                "Paid calls write a ledger row and an audit row atomically.",
              ]}
            />
            <Callout title="Current HTTP shape">
              The implemented <Mono>pay_url</Mono> path uses GET for the probe
              and retry. POST support is a future extension, not current
              behavior.
            </Callout>
          </DocSection>

          <DocSection
            id="policy"
            eyebrow="08"
            title="Budgets, Ledger, And Audit Proof"
          >
            <div className="grid gap-4 md:grid-cols-2">
              <Fact
                title="Budget table"
                body="One default agent row stores daily_usd and per_call_usd. Updates are audited."
              />
              <Fact
                title="Daily spend"
                body="The repo sums ledger_entry.amount_usdc for rows since UTC midnight."
              />
              <Fact
                title="Critical section"
                body="pay_url holds a Tokio mutex while checking budget, settling, and recording the result."
              />
              <Fact
                title="Audit trail"
                body="Rejected calls, no-payment calls, budget updates, and paid calls are visible through agentspay_audit_log."
              />
            </div>
            <CodePanel
              title="SQLite state"
              code={`wallet
budget
policy
ledger_entry
audit_log
seaql_migrations`}
            />
          </DocSection>

          <DocSection id="devnet" eyebrow="09" title="Devnet Funding">
            <p>
              Devnet settlement needs two balances on the signer: SOL for fees
              and USDC for the SPL transfer. Circle&apos;s USDC faucet requires
              a browser captcha, so AgentsPay returns instructions instead of
              trying to auto-fund.
            </p>
            <div className="grid gap-4 md:grid-cols-2">
              <LinkBox
                title="SOL faucet"
                href="https://faucet.solana.com"
                body="Use the same pubkey returned by agentspay_topup_info."
              />
              <LinkBox
                title="USDC faucet"
                href="https://faucet.circle.com"
                body="Select Solana Devnet, paste pubkey, request USDC."
              />
            </div>
            <CodePanel
              title="Local sandbox mode"
              code={`AGENTSPAY_NETWORK=sandbox \\
AGENTSPAY_DATABASE_URL=sqlite:///tmp/agentspay-sandbox.db?mode=rwc \\
./target/release/agentspay-mcp`}
            />
          </DocSection>

          <DocSection id="docker" eyebrow="10" title="Website Demo Stack">
            <p>
              The public demo does not reimplement the product in JavaScript.
              Browser requests go through a Rust shim that spawns the same MCP
              binary per call.
            </p>
            <Flow>
              <span>Browser</span>
              <span>Next.js API proxy</span>
              <span>web-shim</span>
              <span>agentspay-mcp</span>
              <span>paid-endpoint</span>
              <span>Solana RPC</span>
            </Flow>
            <div className="grid gap-4 md:grid-cols-2">
              <Fact
                title="Sandbox sessions"
                body="POST /api/sandbox/session creates an isolated tempdir, keypair, and SQLite ledger for 30 minutes."
              />
              <Fact
                title="Sandbox calls"
                body="POST /api/sandbox/call is limited to 60 calls per minute per session."
              />
              <Fact
                title="Devnet trigger"
                body="POST /api/devnet/trigger is limited to 1 call per IP per hour."
              />
              <Fact
                title="Latest proof"
                body="GET /api/stats/latest-tx returns the most recent settlement for 24 hours and hydrates from SQLite on restart."
              />
            </div>
            <div className="mt-5 space-y-3">
              <CodeBlock value="cp docker/.env.example docker/.env" />
              <CodeBlock value="docker compose -f docker/docker-compose.yml -f docker/docker-compose.local.yml up --build" />
            </div>
          </DocSection>

          <DocSection id="env" eyebrow="11" title="Environment Variables">
            <div className="overflow-x-auto rounded-md border border-border">
              <div className="min-w-[900px]">
                <div className="grid grid-cols-[1fr_1fr_1.6fr] border-b border-border-subtle bg-bg-elev px-4 py-3 font-mono text-xs text-fg-muted">
                  <span>Variable</span>
                  <span>Default</span>
                  <span>Purpose</span>
                </div>
                {ENV_VARS.map(([name, value, purpose]) => (
                  <div
                    key={name}
                    className="grid grid-cols-[1fr_1fr_1.6fr] gap-3 border-b border-border-subtle px-4 py-3 text-xs last:border-b-0"
                  >
                    <span className="font-mono text-syntax-key">{name}</span>
                    <span className="font-mono text-fg-muted">{value}</span>
                    <span className="text-fg-muted">{purpose}</span>
                  </div>
                ))}
              </div>
            </div>
          </DocSection>

          <DocSection id="security" eyebrow="12" title="Security Model">
            <p className="mb-5 max-w-3xl text-sm leading-relaxed text-fg-muted">
              The agent{" "}
              <span className="text-fg">cannot drain your wallet</span>. The
              full threat model, every finding, and the adversarial test suite
              live in{" "}
              <a
                href="https://github.com/h4ckm1n-dev/agentspay/blob/main/SECURITY-AUDIT.md"
                className="text-accent underline-offset-4 hover:underline"
              >
                SECURITY-AUDIT.md
              </a>
              . The regression suite runs on every push:{" "}
              <code className="font-mono text-fg">46 Rust tests</code> (7 SSRF,
              6 adversarial x402, 3 origin-guard, plus the rest of the
              workspace) and{" "}
              <code className="font-mono text-fg">10 TypeScript tests</code>{" "}
              covering the SDK error classifier. Highlights below.
            </p>
            <div className="grid gap-4 md:grid-cols-2">
              <Fact
                title="Per-call + daily caps"
                body="agentspay_pay_url checks both caps before signing. Even an attacker-controlled URL can extract at most per_call_usd per call and daily_usd per day."
              />
              <Fact
                title="SSRF guard"
                body="URLs that resolve to loopback, RFC1918, link-local (incl. AWS/GCP IMDS at 169.254.169.254), CGNAT, or IPv6 ULA are rejected. Opt-out via AGENTSPAY_ALLOW_PRIVATE_HOSTS=1 for local dev."
              />
              <Fact
                title="Asset + decimals validated"
                body="A malicious x402 seller cannot inflate the transfer by quoting funky decimals or a non-USDC mint. Validators reject decimals != 6 and asset != USDC mint in real-signing modes."
              />
              <Fact
                title="1 MiB body cap"
                body="Both the 402 probe and the post-payment retry read at most 1 MiB. No OOM from an attacker streaming gigabytes."
              />
              <Fact
                title="Local key custody, 0600"
                body="The signer is generated locally at ~/.agentspay/keypair.json with owner-only permissions on Unix. Never logged, never sent over the wire."
              />
              <Fact
                title="Non-root containers"
                body="All three Docker images run as uid 10001 (agentspay) or uid 1000 (node). No root inside any container."
              />
              <Fact
                title="Real-IP rate limits"
                body="Public web-shim rate-limits on the real client IP via X-Forwarded-For (Caddy strips client-provided values). Per-IP, not global."
              />
              <Fact
                title="Origin guard"
                body="Mutating shim endpoints require an allowlisted Origin header when AGENTSPAY_ALLOWED_ORIGINS is set. Defense in depth against cross-origin CSRF-like attacks."
              />
            </div>
            <Callout title="Trust boundary">
              The MCP host is the local auth boundary for v0.3. There is no
              multi-tenant server, no dashboard auth, no webhook delivery, and
              no production custody service in this release. Mainnet is gated
              behind a v0.5 compliance review.
            </Callout>
          </DocSection>

          <DocSection id="troubleshooting" eyebrow="13" title="Troubleshooting">
            <div className="space-y-3">
              {TROUBLE.map((item) => (
                <Trouble key={item.symptom} {...item} />
              ))}
            </div>
          </DocSection>

          <DocSection id="commands" eyebrow="14" title="Developer Commands">
            <div className="grid gap-4 md:grid-cols-2">
              <CodePanel
                title="Rust"
                code={`cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check`}
              />
              <CodePanel
                title="Frontend"
                code={`pnpm install
pnpm -r typecheck
pnpm --filter frontend lint
pnpm --filter frontend build`}
              />
              <CodePanel
                title="Native smoke"
                code={`./scripts/devnet-smoke-test.sh
cargo run -p agentspay-paid-endpoint-demo
AGENTSPAY_NETWORK=sandbox cargo run -p agentspay-mcp`}
              />
              <CodePanel
                title="Docker web image"
                code={`docker compose -f docker/docker-compose.yml \\
  -f docker/docker-compose.local.yml build web`}
              />
            </div>
          </DocSection>
        </div>
      </div>
      <Footer />
    </main>
  );
}

function DocSection({
  id,
  eyebrow,
  title,
  children,
}: {
  id: string;
  eyebrow: string;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <section
      id={id}
      className="scroll-mt-24 border-t border-border-subtle pt-10 first:border-t-0 first:pt-0"
    >
      <p className="mb-2 font-mono text-xs text-accent">{eyebrow}</p>
      <h2 className="mb-4 text-2xl font-semibold text-fg sm:text-3xl">
        {title}
      </h2>
      <div className="space-y-5 text-sm leading-relaxed text-fg-muted">
        {children}
      </div>
    </section>
  );
}

function ToolCard({ tool }: { tool: (typeof TOOL_REFERENCE)[number] }) {
  return (
    <article className="tool-panel p-4 sm:p-5">
      <div className="mb-4 flex flex-col justify-between gap-2 sm:flex-row">
        <div>
          <h3 className="font-mono text-sm text-fg">{tool.name}</h3>
          <p className="mt-1 text-xs text-fg-muted">{tool.intent}</p>
        </div>
        <span className="font-mono text-xs text-accent">{tool.call}</span>
      </div>
      <div className="grid gap-4 lg:grid-cols-[1fr_1.2fr]">
        <div className="min-w-0">
          <h4 className="mb-2 text-xs font-semibold uppercase text-fg">Args</h4>
          <CodePanel title="" code={tool.args} compact />
          <h4 className="mb-2 mt-4 text-xs font-semibold uppercase text-fg">
            Returns
          </h4>
          <ul className="space-y-1 text-xs">
            {tool.returns.map((item) => (
              <li key={item}>{item}</li>
            ))}
          </ul>
        </div>
        <CodePanel title="Example response" code={tool.response} />
      </div>
    </article>
  );
}

function Fact({ title, body }: { title: string; body: string }) {
  return (
    <div className="quiet-panel p-4">
      <h3 className="mb-2 text-sm font-semibold text-fg">{title}</h3>
      <p className="text-xs leading-relaxed text-fg-muted">{body}</p>
    </div>
  );
}

function Callout({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div className="rounded-lg border border-accent/30 bg-accent/10 p-4">
      <h3 className="mb-2 text-sm font-semibold text-fg">{title}</h3>
      <p className="text-xs leading-relaxed text-fg-muted">{children}</p>
    </div>
  );
}

function StepList({ items }: { items: readonly string[] }) {
  return (
    <ol className="space-y-3">
      {items.map((item, index) => (
        <li key={item} className="flex gap-3">
          <span className="mt-0.5 grid h-6 w-6 shrink-0 place-items-center rounded-md border border-accent/20 bg-accent/10 font-mono text-xs text-accent">
            {index + 1}
          </span>
          <span>{item}</span>
        </li>
      ))}
    </ol>
  );
}

function Flow({ children }: { children: React.ReactNode }) {
  const items = React.Children.toArray(children);
  return (
    <div className="flex flex-wrap items-center gap-2 rounded-lg border border-border bg-bg-deep p-3 font-mono text-xs">
      {items.map((item, index) => (
        <div key={index} className="flex items-center gap-2">
          <span className="rounded bg-bg-elev px-2.5 py-1.5 text-fg">
            {item}
          </span>
          {index < items.length - 1 && (
            <span className="text-fg-faint" aria-hidden>
              -&gt;
            </span>
          )}
        </div>
      ))}
    </div>
  );
}

function CodePanel({
  title,
  code,
  compact = false,
}: {
  title: string;
  code: string;
  compact?: boolean;
}) {
  return (
    <div className="min-w-0 overflow-hidden rounded-lg border border-border bg-bg-deep shadow-[inset_0_1px_0_rgba(255,255,255,0.03)]">
      {title.length > 0 && (
        <div className="border-b border-border-subtle bg-bg-elev px-3 py-2 font-mono text-xs text-fg-muted">
          {title}
        </div>
      )}
      <pre
        className={`overflow-x-auto whitespace-pre-wrap p-3 font-mono text-xs leading-relaxed text-syntax-string ${
          compact ? "min-h-0" : "min-h-[84px]"
        }`}
      >
        {code}
      </pre>
    </div>
  );
}

function Host({ title, code }: { title: string; code: string }) {
  return (
    <div className="quiet-panel p-4">
      <h3 className="mb-3 text-sm font-semibold text-fg">{title}</h3>
      <pre className="overflow-x-auto whitespace-pre-wrap rounded bg-bg-deep p-3 font-mono text-xs leading-relaxed text-syntax-string">
        {code}
      </pre>
    </div>
  );
}

function LinkBox({
  title,
  href,
  body,
}: {
  title: string;
  href: string;
  body: string;
}) {
  return (
    <a
      href={href}
      className="quiet-panel block p-4 transition hover:border-accent/50"
    >
      <h3 className="mb-2 text-sm font-semibold text-fg">{title}</h3>
      <p className="text-xs leading-relaxed text-fg-muted">{body}</p>
      <p className="mt-3 font-mono text-xs text-accent">{href}</p>
    </a>
  );
}

function Trouble({
  symptom,
  cause,
  fix,
}: {
  symptom: string;
  cause: string;
  fix: string;
}) {
  return (
    <div className="quiet-panel p-4">
      <h3 className="mb-2 text-sm font-semibold text-fg">{symptom}</h3>
      <p className="text-xs leading-relaxed text-fg-muted">
        <span className="text-fg">Cause:</span> {cause}
      </p>
      <p className="mt-2 text-xs leading-relaxed text-fg-muted">
        <span className="text-fg">Fix:</span> {fix}
      </p>
    </div>
  );
}

function Mono({ children }: { children: React.ReactNode }) {
  return <span className="font-mono text-fg">{children}</span>;
}
