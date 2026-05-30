import Link from "next/link";
import { ArrowRight, Database, ShieldCheck } from "lucide-react";
import { LiveReceiptDeck } from "@/components/proof/LiveReceiptDeck";
import { Reveal } from "@/components/ui/Reveal";

export function ProofStrip() {
  return (
    <section className="border-b border-border-subtle bg-black/20 py-14 sm:py-16">
      <div className="section-shell">
        <div className="mb-7 grid gap-6 lg:grid-cols-[1fr_360px] lg:items-end">
          <Reveal>
            <div className="section-kicker">
              <Database className="h-3.5 w-3.5 text-accent" aria-hidden />
              Live proof
            </div>
            <h2 className="max-w-3xl text-3xl font-bold leading-tight sm:text-4xl lg:text-[2.75rem]">
              The product surface is the receipt trail.
            </h2>
            <p className="mt-3 max-w-2xl text-sm leading-6 text-fg-muted sm:text-base">
              No static screenshots. The homepage reads the same devnet ledger
              as the proof page and rotates recent Solscan-confirmed agent
              payments as they land.
            </p>
          </Reveal>
          <Reveal delay={0.08} className="quiet-panel p-4">
            <div className="flex gap-3">
              <span className="grid h-10 w-10 shrink-0 place-items-center rounded-md border border-accent/20 bg-accent/10 text-accent">
                <ShieldCheck className="h-5 w-5" aria-hidden />
              </span>
              <div>
                <p className="font-mono text-xs uppercase text-fg-muted">
                  Audit invariant
                </p>
                <p className="mt-1 text-sm leading-6 text-fg">
                  A signed payment creates a ledger row and a Solscan link.
                </p>
              </div>
            </div>
            <Link href="/proof" className="mt-4 button-secondary w-full">
              View ledger
              <ArrowRight className="h-4 w-4" aria-hidden />
            </Link>
          </Reveal>
        </div>
        <LiveReceiptDeck variant="strip" limit={12} visibleCount={3} />
      </div>
    </section>
  );
}
