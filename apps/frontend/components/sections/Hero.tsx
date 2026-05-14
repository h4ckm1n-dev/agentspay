import Link from "next/link";
import { LiveTxBadge } from "@/components/ui/LiveTxBadge";

export function Hero() {
  return (
    <section className="px-6 pt-24 pb-16 max-w-3xl mx-auto">
      <p className="text-xs uppercase tracking-[0.12em] text-fg-muted mb-4">
        AGENTSPAY · v0.3
      </p>
      <h1 className="text-4xl sm:text-5xl font-semibold leading-[1.05] tracking-tight">
        Give your AI agent a<br />
        budget-controlled wallet.
      </h1>
      <p className="text-fg-muted mt-5 text-base sm:text-lg max-w-2xl">
        One MCP install. Real Solana settlement. Per-call + daily caps enforced
        before the chain — your agent literally cannot drain your wallet.
      </p>
      <div className="flex flex-wrap gap-3 mt-8">
        <Link
          href="#install"
          className="bg-white text-black rounded-md px-4 py-2.5 text-sm font-semibold hover:bg-fg transition"
        >
          Install in Claude Code
        </Link>
        <Link
          href="#demo"
          className="border border-border text-fg rounded-md px-4 py-2.5 text-sm font-medium hover:bg-bg-elev transition"
        >
          See live devnet demo →
        </Link>
      </div>
      <div className="mt-10">
        <LiveTxBadge />
      </div>
    </section>
  );
}
