import {
  Binary,
  Braces,
  FileClock,
  LockKeyhole,
  Route,
} from "lucide-react";

const STEPS = [
  {
    title: "Host invokes MCP",
    body: "Claude Code, Cursor, Cline, or Zed calls one of five stdio JSON-RPC tools. The agent talks to the local binary, not a hosted wallet API.",
    icon: Binary,
  },
  {
    title: "Policy blocks bad spend",
    body: "max_amount_usdc, per-call cap, daily cap, SSRF, asset mint, and decimals are checked before a Solana transaction exists.",
    icon: LockKeyhole,
  },
  {
    title: "x402 gets retried with proof",
    body: "AgentsPay probes the URL, parses the 402 requirements, signs a devnet USDC transfer, and retries with X-Payment.",
    icon: Route,
  },
  {
    title: "Ledger records the outcome",
    body: "Paid, rejected, and no-payment calls are written into SQLite so the SDK, CLI, MCP host, and proof page all see the same truth.",
    icon: FileClock,
  },
] as const;

const CONTRACT = [
  "snake_case response shapes",
  "one subprocess per call",
  "SeaORM ledger writes",
  "typed SDK errors",
] as const;

export function HowItWorks() {
  return (
    <section className="section-shell py-16 sm:py-20">
      <div className="grid gap-8 lg:grid-cols-[0.8fr_1.2fr] lg:items-start">
        <div>
          <div className="section-kicker">
            <Braces className="h-3.5 w-3.5 text-accent" aria-hidden />
            Runtime contract
          </div>
          <h2 className="max-w-2xl text-3xl font-semibold leading-tight sm:text-4xl">
            Small payment path, strict enough for autonomous agents.
          </h2>
          <p className="mt-4 max-w-xl text-sm leading-6 text-fg-muted sm:text-base">
            The implementation is deliberately narrow: a local signer, a budget
            policy, x402 settlement, and receipts a developer can audit from the
            source or the browser.
          </p>
          <div className="mt-6 grid gap-2 sm:grid-cols-2 lg:grid-cols-1">
            {CONTRACT.map((item) => (
              <div key={item} className="code-chip">
                {item}
              </div>
            ))}
          </div>
        </div>

        <div className="grid gap-3 md:grid-cols-2">
          {STEPS.map((step, index) => {
            const Icon = step.icon;
            return (
              <article
                key={step.title}
                className="quiet-panel min-h-[210px] p-5 transition hover:border-accent/40 hover:bg-bg-panel/70"
              >
                <div className="mb-5 flex items-center justify-between">
                  <span className="grid h-10 w-10 place-items-center rounded-md border border-border bg-bg-deep text-accent">
                    <Icon className="h-5 w-5" aria-hidden />
                  </span>
                  <span className="font-mono text-xs text-fg-faint">
                    0{index + 1}
                  </span>
                </div>
                <h3 className="text-base font-semibold text-fg">
                  {step.title}
                </h3>
                <p className="mt-3 text-sm leading-6 text-fg-muted">
                  {step.body}
                </p>
              </article>
            );
          })}
        </div>
      </div>
    </section>
  );
}
