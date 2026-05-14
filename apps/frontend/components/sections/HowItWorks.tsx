const CARDS = [
  {
    n: "1",
    title: "MCP host calls a tool",
    body: "Claude Code, Cursor, or Cline invokes one of the 5 tools over MCP stdio JSON-RPC. Your agent talks to the local binary, never to a hosted service it doesn't control.",
  },
  {
    n: "2",
    title: "Budget check before signature",
    body: "Per-call cap and rolling-daily cap are enforced before any keypair touches the transaction. A tokio Mutex serializes the critical section so two parallel calls can't both pass against a stale view.",
  },
  {
    n: "3",
    title: "On-chain settlement",
    body: "SPL transfer_checked signed locally, base64-encoded, sent in the X-Payment header, settled by the upstream x402 server through Solana devnet RPC. The signature comes back in X-Payment-Response.",
  },
];

export function HowItWorks() {
  return (
    <section className="px-6 py-16 max-w-3xl mx-auto border-t border-border-subtle">
      <p className="text-xs uppercase tracking-[0.12em] text-accent mb-4 font-mono">
        §4 · HOW IT WORKS
      </p>
      <h2 className="text-2xl sm:text-3xl font-semibold tracking-tight mb-6">
        Three steps between a tool call and an on-chain receipt.
      </h2>
      <div className="grid sm:grid-cols-3 gap-4">
        {CARDS.map((c) => (
          <div
            key={c.n}
            className="bg-bg-elev border border-border rounded-md p-4"
          >
            <div className="text-xs font-mono text-accent mb-2">{c.n}</div>
            <h3 className="font-semibold text-fg text-sm mb-2">{c.title}</h3>
            <p className="text-fg-muted text-xs leading-relaxed">{c.body}</p>
          </div>
        ))}
      </div>
    </section>
  );
}
