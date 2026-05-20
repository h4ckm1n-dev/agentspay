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
      <section className="page-hero">
        <div className="mx-auto max-w-7xl">
          <div className="section-kicker">Audit proof</div>
          <h1 className="max-w-3xl text-4xl font-semibold leading-tight sm:text-6xl">
            Every bot-made devnet payment, inspectable.
          </h1>
          <p className="mt-4 max-w-2xl text-sm leading-relaxed text-fg-muted sm:text-base">
            These entries are pulled from the AgentsPay devnet ledger and link
            directly to Solscan. The public view shows up to 20 pages of recent
            receipts so the proof stays readable as the bot produces more
            payments.
          </p>
        </div>
      </section>
      <section className="mx-auto max-w-7xl px-5 py-10 sm:px-6 lg:px-8">
        <ProofLedger />
      </section>
      <Footer />
    </main>
  );
}
