import type { Metadata } from "next";
import { ProofLedger } from "@/components/proof/ProofLedger";
import { Footer } from "@/components/sections/Footer";
import { BreadcrumbStructuredData } from "@/components/seo/PageStructuredData";

export const metadata: Metadata = {
  title: "Audit proof — Solscan-confirmed settlement ledger",
  description:
    "Every AgentsPay settlement is recorded on Solana devnet with a permanent Solscan-verifiable signature. Browse the live ledger of agent-signed USDC payments.",
  alternates: { canonical: "/proof" },
  openGraph: {
    title: "AgentsPay proof — Solscan-confirmed receipts",
    description:
      "Permanent Solana devnet records of agent-signed USDC settlements through AgentsPay.",
    url: "/proof",
    type: "website",
  },
  twitter: {
    card: "summary_large_image",
    title: "AgentsPay proof ledger",
    description: "Solscan-verifiable settlement receipts, signed by AI agents.",
  },
};

export default function ProofPage() {
  return (
    <main>
      <BreadcrumbStructuredData
        trail={[
          { name: "Home", url: "/" },
          { name: "Proof", url: "/proof" },
        ]}
      />
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
