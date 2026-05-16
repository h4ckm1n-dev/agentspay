import { LiveDemo } from "@/components/sections/LiveDemo";
import { Footer } from "@/components/sections/Footer";

export default function DemoPage() {
  return (
    <main className="min-h-screen">
      <section className="border-b border-border-subtle px-6 py-10">
        <div className="mx-auto max-w-6xl">
          <p className="mb-3 font-mono text-xs uppercase tracking-[0.12em] text-accent">
            TERMINAL
          </p>
          <h1 className="text-3xl font-semibold tracking-tight sm:text-5xl">
            Run the MCP wallet from the browser.
          </h1>
          <p className="mt-4 max-w-2xl text-sm leading-relaxed text-fg-muted sm:text-base">
            The sandbox tab gets an isolated ledger. The devnet tab signs a
            real SPL USDC transfer through the same `agentspay-mcp` binary.
          </p>
        </div>
      </section>
      <LiveDemo />
      <Footer />
    </main>
  );
}
