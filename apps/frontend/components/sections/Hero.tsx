import Link from "next/link";
import { ArrowRight, Play, Terminal } from "lucide-react";
import { LiveReceiptTicker } from "@/components/proof/LiveReceiptDeck";
import { Reveal } from "@/components/ui/Reveal";
import { Spotlight } from "@/components/ui/Spotlight";
import { CountUp } from "@/components/ui/CountUp";
import { KineticHeadline } from "@/components/ui/KineticHeadline";
import { MagneticLink } from "@/components/ui/MagneticLink";
import { AuditReceipt } from "@/components/sections/AuditReceipt";

const METRICS = [
  { label: "Per-call cap", value: "checked first", glyph: "≤" },
  { label: "Daily policy", value: "ledger-backed", glyph: "∑" },
  { label: "Settlement", value: "USDC devnet", glyph: "◎" },
  { label: "Receipts", value: "Solscan links", glyph: "✓" },
] as const;

const STATS = [
  {
    value: 2,
    label: "caps before signing",
    accent: true,
    prefix: "",
    suffix: "",
  },
  { value: 5, label: "MCP tools", accent: false, prefix: "", suffix: "" },
  {
    value: 46,
    label: "Rust tests pass",
    accent: false,
    prefix: "",
    suffix: "",
  },
  { value: 100, label: "cold start", accent: false, prefix: "~", suffix: "ms" },
] as const;

export function Hero() {
  return (
    <section className="relative isolate overflow-hidden border-b border-border-subtle">
      <div className="aurora animate-aurora" aria-hidden />
      <Spotlight />
      <div className="section-shell relative py-16 sm:py-20 lg:py-24">
        <div className="grid items-center gap-12 lg:grid-cols-[1.05fr_0.95fr]">
          <div>
            <Reveal>
              <span className="section-kicker">
                <span className="h-1.5 w-1.5 rounded-full bg-accent shadow-[0_0_14px_rgba(16,185,129,0.85)]" />
                Local MCP wallet · Solana devnet
              </span>
            </Reveal>

            <KineticHeadline
              className="mt-2 max-w-[15ch] text-5xl font-black leading-[0.95] tracking-tightest text-fg sm:text-6xl lg:text-7xl"
              lines={[
                "Your agent can spend.",
                <>
                  It{" "}
                  <span className="text-gradient-emerald animate-gradient">
                    can&rsquo;t drain
                  </span>{" "}
                  your wallet.
                </>,
              ]}
            />

            <Reveal delay={0.1}>
              <p className="mt-7 max-w-xl text-base leading-7 text-fg-muted sm:text-lg">
                Give Claude Code, Cursor, Cline, or Zed a budget-capped USDC
                wallet for x402 APIs. Every payment is checked against a
                per-call cap and a daily cap{" "}
                <span className="text-fg">before</span> it is ever signed.
              </p>
            </Reveal>

            <Reveal delay={0.15}>
              <div className="mt-8 flex flex-wrap items-center gap-3">
                <MagneticLink
                  href="#install"
                  className="cta-glow inline-flex min-h-11 items-center justify-center gap-2 rounded-xl2 px-5 py-2.5 text-sm font-semibold text-bg-deep"
                >
                  <Terminal className="h-4 w-4" aria-hidden />
                  Install in Claude Code
                </MagneticLink>
                <Link href="/demo" className="button-secondary">
                  <Play className="h-4 w-4" aria-hidden />
                  Run the demo
                </Link>
                <Link
                  href="/docs"
                  className="inline-flex min-h-11 items-center gap-2 px-2 text-sm font-medium text-fg-muted transition hover:text-fg"
                >
                  Read docs
                  <ArrowRight className="h-4 w-4" aria-hidden />
                </Link>
              </div>
            </Reveal>

            <Reveal delay={0.2}>
              <div className="mt-10 grid gap-2.5 sm:grid-cols-2 lg:grid-cols-4">
                {METRICS.map((metric) => (
                  <div key={metric.label} className="metric-pill">
                    <span className="grid h-8 w-8 shrink-0 place-items-center rounded-lg border border-accent/20 bg-accent/10 font-mono text-sm text-accent">
                      {metric.glyph}
                    </span>
                    <span className="min-w-0">
                      <span className="block font-medium text-fg">
                        {metric.label}
                      </span>
                      <span className="block truncate text-fg-faint">
                        {metric.value}
                      </span>
                    </span>
                  </div>
                ))}
              </div>
            </Reveal>
          </div>

          <Reveal delay={0.15} y={24}>
            <AuditReceipt />
          </Reveal>
        </div>

        <Reveal delay={0.1}>
          <div className="mt-12 max-w-3xl">
            <LiveReceiptTicker />
          </div>
        </Reveal>

        <Reveal delay={0.1}>
          <div className="mt-12 grid grid-cols-2 gap-px overflow-hidden rounded-xl2 border border-border-subtle bg-border-subtle sm:grid-cols-4">
            {STATS.map((stat) => (
              <div key={stat.label} className="bg-bg px-5 py-6">
                <CountUp
                  value={stat.value}
                  prefix={stat.prefix}
                  suffix={stat.suffix}
                  className={`font-display text-4xl font-black tracking-tightest ${
                    stat.accent ? "text-accent" : "text-fg"
                  }`}
                  suffixClassName="text-xl align-baseline"
                />
                <div className="mt-1.5 font-mono text-xs uppercase tracking-[0.08em] text-fg-faint">
                  {stat.label}
                </div>
              </div>
            ))}
          </div>
        </Reveal>
      </div>
    </section>
  );
}
