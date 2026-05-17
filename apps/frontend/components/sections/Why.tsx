const CARDS = [
  {
    title: "vs hardcoded keys",
    body: "Your agent cannot drain your wallet. Per-call and daily caps are checked before pay_url signs anything. A hardcoded API key gives the agent the full credit card; AgentsPay gives it a wallet that says no.",
  },
  {
    title: "vs writing it yourself",
    body: "Skip the SPL-token plumbing, blockhash management, and ATA creation. Five typed tools, audit log baked in, attack surface closed against the obvious vectors (SSRF, decimals inflation, OOM bodies, malicious mint, CSRF).",
  },
  {
    title: "vs Stripe MPP",
    body: "Self-custodial. Open source MIT. Solana, not card rails. The whole stack runs on your laptop with no SaaS dependency and no merchant onboarding.",
  },
  {
    title: "vs Coinbase CDP direct",
    body: "Five MCP tools, not forty REST endpoints. No API key. Drops into Claude Code with one command. The CDP facilitator is still an option under the hood when you want it.",
  },
];

export function Why() {
  return (
    <section className="mx-auto max-w-6xl border-t border-border-subtle px-6 py-16">
      <p className="text-xs uppercase tracking-[0.12em] text-accent mb-4 font-mono">
        WHY
      </p>
      <h2 className="text-2xl sm:text-3xl font-semibold tracking-tight mb-2">
        Built for indie devs in Claude Code, not enterprise procurement.
      </h2>
      <p className="mb-6 max-w-2xl text-sm text-fg-muted">
        Audited surface. Typed errors. Real settlement. No middleman.
      </p>
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        {CARDS.map((c) => (
          <div
            key={c.title}
            className="bg-bg-elev border border-border rounded-md p-4"
          >
            <h3 className="font-semibold text-fg text-sm mb-2">{c.title}</h3>
            <p className="text-fg-muted text-xs leading-relaxed">{c.body}</p>
          </div>
        ))}
      </div>
    </section>
  );
}
