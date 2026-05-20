import Link from "next/link";
import {
  ArrowRight,
  Braces,
  CircleDollarSign,
  LockKeyhole,
  Play,
  ReceiptText,
  ShieldCheck,
  Terminal,
  Wallet,
} from "lucide-react";
import { LiveReceiptTicker } from "@/components/proof/LiveReceiptDeck";

const METRICS = [
  { label: "Per-call cap", value: "checked first", icon: LockKeyhole },
  { label: "Daily policy", value: "ledger-backed", icon: ShieldCheck },
  { label: "Settlement", value: "USDC devnet", icon: Wallet },
  { label: "Receipts", value: "Solscan links", icon: ReceiptText },
] as const;

const FLOW = [
  "MCP call",
  "budget gate",
  "x402 quote",
  "SPL transfer",
  "audit row",
] as const;

const CODE_LINES = [
  ["tool", "agentspay_pay_url"],
  ["max_amount_usdc", "0.50"],
  ["policy", "per_call <= 1.00"],
  ["network", "solana-devnet"],
  ["status", "paid"],
] as const;

export function Hero() {
  return (
    <section className="relative isolate overflow-hidden border-b border-border-subtle">
      <RuntimeBackdrop />
      <div className="section-shell relative flex min-h-[78svh] items-center py-14 sm:py-16 lg:py-20">
        <div className="max-w-4xl">
          <div className="section-kicker">
            <span className="h-1.5 w-1.5 rounded-full bg-accent shadow-[0_0_18px_rgba(20,184,166,0.75)]" />
            Local MCP wallet for agent payments
          </div>

          <h1 className="max-w-4xl text-5xl font-semibold leading-none text-fg sm:text-7xl lg:text-8xl">
            AgentsPay
          </h1>
          <p className="mt-6 max-w-2xl text-base leading-8 text-fg-muted sm:text-lg">
            Give Claude Code, Cursor, Cline, or Zed a USDC wallet that can pay
            x402 APIs without handing the agent an unlimited key. Budgets,
            SSRF defense, mint validation, and audit proof sit before every
            signature.
          </p>

          <div className="mt-8 flex flex-wrap gap-3">
            <Link href="#install" className="button-primary">
              <Terminal className="h-4 w-4" aria-hidden />
              Install in Claude Code
            </Link>
            <Link href="/demo" className="button-secondary">
              <Play className="h-4 w-4" aria-hidden />
              Run browser demo
            </Link>
            <Link
              href="/docs"
              className="inline-flex min-h-10 items-center justify-center gap-2 rounded-md px-2 py-2.5 text-sm font-medium text-fg-muted transition hover:text-fg"
            >
              Read docs
              <ArrowRight className="h-4 w-4" aria-hidden />
            </Link>
          </div>

          <div className="mt-9 grid max-w-4xl gap-2 sm:grid-cols-2 lg:grid-cols-4">
            {METRICS.map((metric) => {
              const Icon = metric.icon;
              return (
                <div key={metric.label} className="metric-pill">
                  <span className="grid h-8 w-8 shrink-0 place-items-center rounded-md border border-accent/20 bg-accent/10 text-accent">
                    <Icon className="h-4 w-4" aria-hidden />
                  </span>
                  <span className="min-w-0">
                    <span className="block text-fg">{metric.label}</span>
                    <span className="block truncate text-fg-faint">
                      {metric.value}
                    </span>
                  </span>
                </div>
              );
            })}
          </div>

          <div className="mt-8 max-w-3xl">
            <LiveReceiptTicker />
          </div>
        </div>
      </div>
    </section>
  );
}

function RuntimeBackdrop() {
  return (
    <div className="absolute inset-0 -z-10 overflow-hidden" aria-hidden>
      <div className="absolute inset-0 bg-[linear-gradient(90deg,rgba(7,7,8,0.98)_0%,rgba(7,7,8,0.88)_42%,rgba(7,7,8,0.52)_100%)]" />
      <div className="absolute inset-x-0 bottom-0 h-24 bg-gradient-to-t from-bg to-transparent" />

      <div className="absolute right-[-12rem] top-12 hidden w-[760px] rotate-[-3deg] lg:block">
        <div className="tool-panel overflow-hidden">
          <div className="flex items-center justify-between border-b border-border-subtle bg-bg-elev/75 px-4 py-3 font-mono text-xs text-fg-muted">
            <span>agentspay runtime</span>
            <span className="text-accent">devnet/live</span>
          </div>
          <div className="grid grid-cols-[1fr_1.08fr] gap-px bg-border-subtle">
            <div className="bg-bg-panel/90 p-5">
              <div className="flex items-center gap-2 font-mono text-xs text-fg-muted">
                <Braces className="h-4 w-4 text-accent" aria-hidden />
                policy envelope
              </div>
              <div className="mt-4 space-y-2">
                {CODE_LINES.map(([key, value]) => (
                  <div
                    key={key}
                    className="flex items-center justify-between gap-3 rounded-md border border-border bg-black/30 px-3 py-2 font-mono text-xs"
                  >
                    <span className="text-fg-muted">{key}</span>
                    <span className="truncate text-syntax-string">{value}</span>
                  </div>
                ))}
              </div>
            </div>
            <div className="bg-bg-deep p-5 font-mono text-xs">
              <div className="flex items-center justify-between text-fg-muted">
                <span>x402 retry</span>
                <span className="text-accent">signed</span>
              </div>
              <pre className="mt-4 whitespace-pre-wrap leading-6 text-syntax-string">{`{
  "payment_status": "paid",
  "amount_charged_usdc": "0.10",
  "ledger_entry_id": "baf3...",
  "explorer_url": "solscan.io/tx/..."
}`}</pre>
            </div>
          </div>
        </div>

        <div className="ml-16 mt-4 grid grid-cols-5 gap-2">
          {FLOW.map((item, index) => (
            <div
              key={item}
              className="rounded-lg border border-border bg-bg-panel/70 p-3 shadow-[0_18px_70px_rgba(0,0,0,0.28)]"
            >
              <div className="mb-3 flex items-center justify-between gap-2">
                <span className="font-mono text-[10px] uppercase text-fg-muted">
                  0{index + 1}
                </span>
                {index === 3 ? (
                  <CircleDollarSign
                    className="h-4 w-4 text-accent-gold"
                    aria-hidden
                  />
                ) : (
                  <span className="h-1.5 w-1.5 rounded-full bg-accent" />
                )}
              </div>
              <p className="min-h-8 text-xs leading-4 text-fg">{item}</p>
            </div>
          ))}
        </div>
      </div>

      <div className="absolute bottom-8 right-5 hidden w-[420px] rounded-lg border border-border bg-bg-deep/70 p-4 font-mono text-xs text-fg-muted shadow-[0_24px_90px_rgba(0,0,0,0.32)] backdrop-blur md:block lg:right-28">
        <div className="mb-3 flex items-center justify-between">
          <span>local audit invariants</span>
          <span className="text-accent">pre-sign</span>
        </div>
        {[
          ["ssrf_guard", "pass"],
          ["asset_mint", "USDC"],
          ["decimals", "6"],
        ].map(([label, value]) => (
          <div
            key={label}
            className="grid grid-cols-[1fr_auto] gap-3 border-t border-border-subtle py-2"
          >
            <span>{label}</span>
            <span className="text-syntax-string">{value}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
