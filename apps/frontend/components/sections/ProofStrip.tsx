import Link from "next/link";
import { ArrowRight } from "lucide-react";
import { LiveReceiptDeck } from "@/components/proof/LiveReceiptDeck";

export function ProofStrip() {
  return (
    <section className="mx-auto max-w-6xl px-6 py-14">
      <div className="mb-5 flex flex-col justify-between gap-3 sm:flex-row sm:items-end">
        <div>
          <p className="mb-2 font-mono text-xs uppercase tracking-[0.12em] text-accent">
            LIVE PROOF
          </p>
          <h2 className="text-2xl font-semibold tracking-tight">
            The latest three bot-made devnet receipts.
          </h2>
          <p className="mt-2 max-w-2xl text-sm text-fg-muted">
            Every entry links to Solscan. You don&apos;t take our word for it,
            the chain has the receipts. When the ledger has more receipts the
            cards rotate through the recent set automatically.
          </p>
        </div>
        <Link
          href="/proof"
          className="inline-flex items-center gap-2 text-sm text-fg-muted transition hover:text-fg"
        >
          View ledger
          <ArrowRight className="h-4 w-4" aria-hidden />
        </Link>
      </div>
      <LiveReceiptDeck variant="strip" limit={12} visibleCount={3} />
    </section>
  );
}
