import { CodeBlock } from "@/components/ui/CodeBlock";

export function Install() {
  return (
    <section
      id="install"
      className="mx-auto max-w-6xl border-t border-border-subtle px-6 py-16"
    >
      <p className="text-xs uppercase tracking-[0.12em] text-accent mb-4 font-mono">
        INSTALL
      </p>
      <h2 className="text-2xl sm:text-3xl font-semibold tracking-tight mb-6">
        One command. Five MCP tools. Real settlement.
      </h2>
      <div className="max-w-3xl">
        <CodeBlock value="cargo build --release -p agentspay-mcp && claude mcp add agentspay ./target/release/agentspay-mcp" />
      </div>
      <p className="text-fg-muted text-sm mt-4">
        Cursor, Cline, and Zed use the same binary. The full host config and
        env var reference lives in <a className="text-fg hover:text-accent" href="/docs">docs</a>.
      </p>
    </section>
  );
}
