import Link from "next/link";
import { PROOF_RECORDS, shortSignature, solscanUrl } from "@/lib/proof-data";

export function ProofStrip() {
  return (
    <section className="mx-auto max-w-6xl px-6 py-12">
      <div className="mb-5 flex flex-col justify-between gap-3 sm:flex-row sm:items-end">
        <div>
          <p className="mb-2 font-mono text-xs uppercase tracking-[0.12em] text-accent">
            LIVE PROOF
          </p>
          <h2 className="text-2xl font-semibold tracking-tight">
            Three Solscan-confirmed receipts.
          </h2>
        </div>
        <Link href="/proof" className="text-sm text-fg-muted transition hover:text-fg">
          View ledger
        </Link>
      </div>
      <div className="grid gap-3 md:grid-cols-3">
        {PROOF_RECORDS.map((record) => (
          <a
            key={record.signature}
            href={solscanUrl(record.signature)}
            className="rounded-md border border-border bg-bg-elev p-4 transition hover:border-accent/50"
          >
            <div className="mb-3 flex items-center justify-between gap-3">
              <span className="font-mono text-sm text-fg">{record.symbol}</span>
              <span className="font-mono text-xs text-accent">
                {record.amountUsdc} USDC
              </span>
            </div>
            <p className="font-mono text-xs text-syntax-signature">
              {shortSignature(record.signature)}
            </p>
            <p className="mt-3 text-xs leading-relaxed text-fg-muted">
              {record.context}
            </p>
          </a>
        ))}
      </div>
    </section>
  );
}
