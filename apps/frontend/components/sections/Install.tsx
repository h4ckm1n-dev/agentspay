import { CodeBlock } from "@/components/ui/CodeBlock";

const PATHS = [
  {
    label: "MCP host",
    sub: "Claude Code, Cursor, Cline, Zed",
    code: "cargo build --release -p agentspay-mcp\nclaude mcp add agentspay ./target/release/agentspay-mcp",
  },
  {
    label: "TypeScript SDK",
    sub: "Node.js apps without an MCP host",
    code: 'pnpm add @agentspay/sdk-js\n\nimport { AgentsPayClient } from "@agentspay/sdk-js";\nconst c = new AgentsPayClient({ network: "solana-devnet" });\nawait c.payUrl({ url, maxAmountUsdc: "0.50" });',
  },
  {
    label: "Command line",
    sub: "Shell scripts and one-off calls",
    code: "pnpm add -g @agentspay/cli\nagentspay balance\nagentspay pay-url <url> --max 0.50",
  },
] as const;

export function Install() {
  return (
    <section
      id="install"
      className="mx-auto max-w-6xl border-t border-border-subtle px-6 py-16"
    >
      <p className="mb-4 font-mono text-xs uppercase tracking-[0.12em] text-accent">
        INSTALL
      </p>
      <h2 className="mb-2 text-2xl font-semibold tracking-tight sm:text-3xl">
        Three ways to use AgentsPay. Same binary, same five tools.
      </h2>
      <p className="mb-8 max-w-3xl text-sm text-fg-muted">
        The MCP server is the primary surface. The SDK and CLI wrap the same
        binary so apps that don&apos;t speak MCP can still use the wallet. Full
        host configs, env vars, and the security model are in the{" "}
        <a className="text-fg hover:text-accent" href="/docs">
          docs
        </a>
        .
      </p>
      <div className="grid gap-4 md:grid-cols-3">
        {PATHS.map((p) => (
          <div
            key={p.label}
            className="rounded-md border border-border bg-bg-elev/30 p-4"
          >
            <div className="mb-3">
              <p className="font-mono text-xs uppercase tracking-[0.1em] text-accent">
                {p.label}
              </p>
              <p className="text-xs text-fg-muted">{p.sub}</p>
            </div>
            <CodeBlock value={p.code} />
          </div>
        ))}
      </div>
    </section>
  );
}
