import type { Metadata } from "next";
import { LiveDemo } from "@/components/sections/LiveDemo";
import { Footer } from "@/components/sections/Footer";
import { BreadcrumbStructuredData } from "@/components/seo/PageStructuredData";

export const metadata: Metadata = {
  title: "Live demo — Sandbox + Solana devnet",
  description:
    "Run the AgentsPay MCP wallet from your browser. Ask for an x402 payment request, then pay it with a real Solana devnet USDC transfer.",
  alternates: { canonical: "/demo" },
  openGraph: {
    title: "AgentsPay live demo — Sandbox + Solana devnet",
    description:
      "Browser demo for the complete x402 loop: request payment, sign with agentspay-mcp, and produce a Solscan-confirmed devnet USDC settlement.",
    url: "/demo",
    type: "website",
  },
  twitter: {
    card: "summary_large_image",
    title: "AgentsPay live demo",
    description:
      "Ask for payment, then trigger a real Solana devnet USDC settlement from the browser.",
  },
};

export default function DemoPage() {
  return (
    <main className="min-h-screen">
      <BreadcrumbStructuredData
        trail={[
          { name: "Home", url: "/" },
          { name: "Demo", url: "/demo" },
        ]}
      />
      <section className="page-hero">
        <div className="mx-auto max-w-7xl">
          <div className="section-kicker">Terminal</div>
          <h1 className="max-w-3xl text-4xl font-semibold leading-tight sm:text-6xl">
            Run the MCP wallet from the browser.
          </h1>
          <p className="mt-4 max-w-2xl text-sm leading-relaxed text-fg-muted sm:text-base">
            The sandbox tab gets an isolated ledger. The devnet tab asks the
            demo provider for an x402 payment request, then signs a real SPL
            USDC transfer through the same `agentspay-mcp` binary.
          </p>
        </div>
      </section>
      <LiveDemo />
      <Footer />
    </main>
  );
}
