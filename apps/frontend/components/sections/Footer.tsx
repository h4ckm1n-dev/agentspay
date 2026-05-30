import { AgentsPayMark } from "@/components/brand/AgentsPayMark";

export function Footer() {
  return (
    <footer className="relative overflow-hidden border-t border-border-subtle bg-black/25">
      <div className="mx-auto flex max-w-7xl flex-col gap-8 px-5 py-10 text-xs text-fg-dim sm:flex-row sm:items-start sm:justify-between sm:px-6 lg:px-8">
        <div className="flex max-w-md gap-3">
          <AgentsPayMark className="h-9 w-9 shrink-0" />
          <div className="space-y-1.5">
            <p className="font-mono text-fg">agentspay</p>
            <p>
              Open source / MIT / Built in <span className="text-fg">Rust</span>{" "}
              + <span className="text-fg">Next.js</span>
            </p>
            <p>
              Status: <span className="text-fg">v0.3, Solana devnet.</span>{" "}
              Mainnet gated behind a v0.5 compliance review.
            </p>
          </div>
        </div>
        <div className="grid grid-cols-2 gap-x-8 gap-y-1.5 sm:grid-cols-3">
          <a
            href="https://github.com/h4ckm1n-dev/agentspay"
            className="hover:text-fg transition"
          >
            GitHub
          </a>
          <a href="/docs" className="hover:text-fg transition">
            Docs
          </a>
          <a href="/proof" className="hover:text-fg transition">
            Proof ledger
          </a>
          <a href="/demo" className="hover:text-fg transition">
            Live demo
          </a>
          <a
            href="https://github.com/h4ckm1n-dev/agentspay/blob/main/SECURITY-AUDIT.md"
            className="hover:text-fg transition"
          >
            Security audit
          </a>
          <a
            href="https://github.com/h4ckm1n-dev/agentspay/blob/main/packages/sdk-js/README.md"
            className="hover:text-fg transition"
          >
            @agentspay/sdk-js
          </a>
          <a
            href="https://github.com/h4ckm1n-dev/agentspay/blob/main/packages/cli/README.md"
            className="hover:text-fg transition"
          >
            @agentspay/cli
          </a>
          <a
            href="https://github.com/h4ckm1n-dev/agentspay/blob/main/Plan.md"
            className="hover:text-fg transition"
          >
            Roadmap
          </a>
        </div>
      </div>
      <div
        aria-hidden
        className="pointer-events-none select-none px-5 pb-1 sm:px-6 lg:px-8"
      >
        <span className="block bg-gradient-to-b from-white/[0.045] to-transparent bg-clip-text font-display text-[clamp(2rem,9vw,7rem)] font-black leading-[0.8] tracking-tightest text-transparent">
          AgentsPay
        </span>
      </div>
    </footer>
  );
}
