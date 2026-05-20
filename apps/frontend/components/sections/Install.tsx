import { Code2, Command, Plug, Terminal } from "lucide-react";
import { CodeBlock } from "@/components/ui/CodeBlock";

const PATHS = [
  {
    label: "MCP host",
    sub: "Claude Code, Cursor, Cline, Zed",
    icon: Terminal,
    code: "cargo build --release -p agentspay-mcp\nclaude mcp add agentspay $PWD/target/release/agentspay-mcp",
  },
  {
    label: "TypeScript SDK",
    sub: "Node.js apps without an MCP host",
    icon: Code2,
    code: 'pnpm add @agentspay/sdk-js\n\nimport { AgentsPayClient } from "@agentspay/sdk-js";\nconst client = new AgentsPayClient({ network: "solana-devnet" });\nawait client.payUrl({ url, maxAmountUsdc: "0.50" });',
  },
  {
    label: "CLI",
    sub: "Shell scripts and smoke tests",
    icon: Command,
    code: "pnpm add -g @agentspay/cli\nagentspay balance\nagentspay pay-url <url> --max 0.50",
  },
] as const;

export function Install() {
  return (
    <section id="install" className="border-y border-border-subtle bg-black/20">
      <div className="section-shell py-16 sm:py-20">
        <div className="grid gap-8 lg:grid-cols-[0.74fr_1.26fr] lg:items-start">
          <div>
            <div className="section-kicker">
              <Plug className="h-3.5 w-3.5 text-accent" aria-hidden />
              Developer install
            </div>
            <h2 className="max-w-xl text-3xl font-semibold leading-tight sm:text-4xl">
              One local binary, three ways to integrate.
            </h2>
            <p className="mt-4 max-w-xl text-sm leading-6 text-fg-muted sm:text-base">
              The MCP server is the product surface. The SDK and CLI wrap the
              same subprocess so your app, terminal, and agent host share one
              wallet boundary and one audit trail.
            </p>
            <a href="/docs" className="mt-6 button-secondary">
              Open full docs
            </a>
          </div>

          <div className="grid gap-4">
            {PATHS.map((path, index) => {
              const Icon = path.icon;
              return (
                <article
                  key={path.label}
                  className="grid gap-4 rounded-lg border border-border bg-bg-panel/70 p-4 md:grid-cols-[220px_1fr]"
                >
                  <div className="flex items-start gap-3">
                    <span className="grid h-10 w-10 shrink-0 place-items-center rounded-md border border-border bg-bg-deep text-accent">
                      <Icon className="h-5 w-5" aria-hidden />
                    </span>
                    <div>
                      <p className="text-sm font-semibold text-fg">
                        {path.label}
                      </p>
                      <p className="mt-1 text-xs leading-5 text-fg-muted">
                        {path.sub}
                      </p>
                      <p className="mt-3 font-mono text-xs text-fg-faint">
                        path 0{index + 1}
                      </p>
                    </div>
                  </div>
                  <CodeBlock value={path.code} />
                </article>
              );
            })}
          </div>
        </div>
      </div>
    </section>
  );
}
