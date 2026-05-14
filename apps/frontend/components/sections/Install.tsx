import { CodeBlock } from "@/components/ui/CodeBlock";

export function Install() {
  return (
    <section id="install" className="px-6 py-16 max-w-3xl mx-auto border-t border-border-subtle">
      <p className="text-xs uppercase tracking-[0.12em] text-accent mb-4 font-mono">
        §2 · INSTALL
      </p>
      <h2 className="text-2xl sm:text-3xl font-semibold tracking-tight mb-6">
        One command. Five MCP tools. Real on-chain settlement.
      </h2>
      <CodeBlock value="claude mcp add agentspay --from github:user/agentspay" />
      <p className="text-fg-muted text-sm mt-4">
        Cursor / Cline / Zed: see the README — manual config takes 30 seconds.
        Or grab a prebuilt binary from the latest GitHub release.
      </p>
    </section>
  );
}
