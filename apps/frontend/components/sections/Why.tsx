const CARDS = [
  {
    title: "vs hardcoded API key",
    body: "Caps + audit trail. Your agent literally cannot drain your OpenAI bill — pay_url refuses any call that would push the day's spend above your budget.",
  },
  {
    title: "vs Stripe MPP",
    body: "Self-custodial. Open source. Solana, not card rails. The whole stack runs on your laptop — no SaaS dependency, no merchant onboarding.",
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
      <h2 className="text-2xl sm:text-3xl font-semibold tracking-tight mb-6">
        Built for indie devs in Claude Code, not enterprise procurement.
      </h2>
      <div className="grid sm:grid-cols-3 gap-4">
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
