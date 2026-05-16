import { ProofLedger } from "@/components/proof/ProofLedger";
import { Footer } from "@/components/sections/Footer";

export default function ProofPage() {
  return (
    <main>
      <section className="border-b border-border-subtle px-6 py-12">
        <div className="mx-auto max-w-6xl">
          <p className="mb-3 font-mono text-xs uppercase tracking-[0.12em] text-accent">
            AUDIT PROOF
          </p>
          <h1 className="max-w-3xl text-3xl font-semibold tracking-tight sm:text-5xl">
            Solscan-confirmed settlement ledger.
          </h1>
          <p className="mt-4 max-w-2xl text-sm leading-relaxed text-fg-muted sm:text-base">
            These entries are devnet receipts signed by `agentspay-mcp`.
            Historical transactions are permanent records. The live row appears
            when the public demo has produced a recent settlement.
          </p>
        </div>
      </section>
      <section className="mx-auto max-w-6xl px-6 py-10">
        <ProofLedger />
      </section>
      <Footer />
    </main>
  );
}
