import type { Metadata } from "next";
import { LiveDemo } from "@/components/sections/LiveDemo";
import { Footer } from "@/components/sections/Footer";
import { BreadcrumbStructuredData } from "@/components/seo/PageStructuredData";

export const metadata: Metadata = {
  title: "Live demo — Sandbox + Solana devnet",
  description:
    "Run the AgentsPay MCP wallet from your browser. The sandbox tab gets an isolated ledger; the devnet tab signs a real SPL USDC transfer through the same agentspay-mcp binary.",
  alternates: { canonical: "/demo" },
  openGraph: {
    title: "AgentsPay live demo — Sandbox + Solana devnet",
    description:
      "One-click browser demo. Click the trigger to produce a Solscan-confirmed devnet USDC settlement signed by the same MCP binary you would install locally.",
    url: "/demo",
    type: "website",
  },
  twitter: {
    card: "summary_large_image",
    title: "AgentsPay live demo",
    description:
      "Trigger a real Solana devnet USDC settlement from the browser.",
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
      <section className="border-b border-border-subtle px-6 py-10">
        <div className="mx-auto max-w-6xl">
          <p className="mb-3 font-mono text-xs uppercase tracking-[0.12em] text-accent">
            TERMINAL
          </p>
          <h1 className="text-3xl font-semibold tracking-tight sm:text-5xl">
            Run the MCP wallet from the browser.
          </h1>
          <p className="mt-4 max-w-2xl text-sm leading-relaxed text-fg-muted sm:text-base">
            The sandbox tab gets an isolated ledger. The devnet tab signs a real
            SPL USDC transfer through the same `agentspay-mcp` binary.
          </p>
        </div>
      </section>
      <LiveDemo />
      <Footer />
    </main>
  );
}
