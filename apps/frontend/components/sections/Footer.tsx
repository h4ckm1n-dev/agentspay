export function Footer() {
  return (
    <footer className="mx-auto mt-16 flex max-w-6xl flex-col gap-6 border-t border-border-subtle px-6 py-10 text-xs text-fg-dim sm:flex-row sm:items-start sm:justify-between">
      <div className="space-y-1.5">
        <p>
          Open source · MIT · Built in <span className="text-fg">Rust</span> +{" "}
          <span className="text-fg">Next.js</span>
        </p>
        <p>
          Status: <span className="text-fg">v0.3, Solana devnet.</span> Mainnet
          gated behind a v0.5 compliance review.
        </p>
      </div>
      <div className="grid grid-cols-2 gap-x-8 gap-y-1.5 sm:grid-cols-3">
        <a
          href="https://github.com/h4ckm1n/agentspay"
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
          href="https://github.com/h4ckm1n/agentspay/blob/main/SECURITY-AUDIT.md"
          className="hover:text-fg transition"
        >
          Security audit
        </a>
        <a
          href="https://github.com/h4ckm1n/agentspay/blob/main/packages/sdk-js/README.md"
          className="hover:text-fg transition"
        >
          @agentspay/sdk-js
        </a>
        <a
          href="https://github.com/h4ckm1n/agentspay/blob/main/packages/cli/README.md"
          className="hover:text-fg transition"
        >
          @agentspay/cli
        </a>
        <a
          href="https://github.com/h4ckm1n/agentspay/blob/main/Plan.md"
          className="hover:text-fg transition"
        >
          Roadmap
        </a>
      </div>
    </footer>
  );
}
