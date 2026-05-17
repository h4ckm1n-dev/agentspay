import Link from "next/link";
import { LiveReceiptTicker } from "@/components/proof/LiveReceiptDeck";

export function Hero() {
  return (
    <section className="relative overflow-hidden border-b border-border-subtle">
      <div
        className="absolute inset-0 bg-cover bg-center opacity-35"
        style={{
          backgroundImage:
            "linear-gradient(90deg, rgba(10,10,11,0.98) 0%, rgba(10,10,11,0.78) 46%, rgba(10,10,11,0.48) 100%), url('https://images.unsplash.com/photo-1518005020951-eccb494ad742?auto=format&fit=crop&w=1800&q=80')",
        }}
        aria-hidden
      />
      <div className="relative mx-auto grid min-h-[72svh] max-w-6xl items-center gap-10 px-6 py-16 md:grid-cols-[1.05fr_0.95fr]">
        <div>
          <p className="mb-4 font-mono text-xs uppercase tracking-[0.12em] text-fg-muted">
            ~/agentspay
          </p>
          <h1 className="max-w-3xl text-4xl font-semibold leading-[1.05] sm:text-6xl">
            AgentsPay
          </h1>
          <p className="mt-5 max-w-2xl text-base text-fg-muted sm:text-lg">
            A USDC wallet your AI agent{" "}
            <span className="text-fg">cannot drain</span>. Per-call and daily
            caps are checked before signing. Settles on Solana in ~2 seconds.
            Drops into Claude Code, Cursor, Cline, or Zed.
          </p>
          <div className="mt-8 flex flex-wrap gap-3">
            <Link
              href="#install"
              className="rounded-md bg-white px-4 py-2.5 text-sm font-semibold text-black transition hover:bg-fg"
            >
              Install in Claude Code
            </Link>
            <Link
              href="/demo"
              className="rounded-md border border-border px-4 py-2.5 text-sm font-medium text-fg transition hover:bg-bg-elev"
            >
              Run the demo
            </Link>
          </div>
          <p className="mt-3 text-xs text-fg-muted">
            Not in Claude Code? Use{" "}
            <code className="font-mono text-fg">@agentspay/sdk-js</code> from
            Node, or <code className="font-mono text-fg">@agentspay/cli</code>{" "}
            from a shell. Same binary, same five tools.
          </p>
          <div className="mt-8 flex flex-wrap items-center gap-x-4 gap-y-2 font-mono text-xs text-fg-muted">
            <span>open source · MIT</span>
            <span className="text-fg-faint">·</span>
            <span>46 Rust + 10 TS tests</span>
            <span className="text-fg-faint">·</span>
            <span>
              security audit (4 CRIT fixed){" "}
              <Link
                href="https://github.com/h4ckm1n/agentspay/blob/main/SECURITY-AUDIT.md"
                className="text-accent underline-offset-4 hover:underline"
              >
                report
              </Link>
            </span>
          </div>
          <div className="mt-8">
            <LiveReceiptTicker />
          </div>
        </div>

        <div className="rounded-md border border-border bg-black/70 font-mono text-xs shadow-[0_24px_80px_rgba(0,0,0,0.45)] backdrop-blur">
          <div className="flex items-center justify-between border-b border-border-subtle bg-bg-elev/80 px-3 py-2">
            <div className="flex items-center gap-2">
              <span className="h-2.5 w-2.5 rounded-full bg-red-400" />
              <span className="h-2.5 w-2.5 rounded-full bg-yellow-400" />
              <span className="h-2.5 w-2.5 rounded-full bg-green-400" />
            </div>
            <span className="text-fg-faint">mcp/v0.3</span>
          </div>
          <div className="space-y-3 p-4 text-fg-muted">
            <p>
              <span className="text-syntax-punct">$ </span>
              <span className="text-fg">agentspay_balance</span>
            </p>
            <pre className="whitespace-pre-wrap text-syntax-string">{`{
  "available_usdc": "19.90",
  "budget_remaining_today_usdc": "24.90",
  "environment": "solana-devnet"
}`}</pre>
            <p>
              <span className="text-syntax-punct">$ </span>
              <span className="text-fg">
                agentspay_pay_url max_amount_usdc=0.50
              </span>
            </p>
            <p className="text-accent">
              paid 0.10 USDC - receipt written to proof ledger
            </p>
          </div>
        </div>
      </div>
    </section>
  );
}
