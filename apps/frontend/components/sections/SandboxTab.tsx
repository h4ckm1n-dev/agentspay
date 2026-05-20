"use client";

import { useState } from "react";
import {
  Activity,
  FileClock,
  PiggyBank,
  WalletCards,
  type LucideIcon,
} from "lucide-react";
import { callTool, ToolName } from "@/lib/api";
import { Terminal, TerminalLine } from "@/components/ui/Terminal";

interface ToolButton {
  tool: ToolName;
  label: string;
  hint: string;
  args: () => object;
  icon: LucideIcon;
}

const BUTTONS: ToolButton[] = [
  {
    tool: "agentspay_balance",
    label: "balance",
    hint: "wallet row",
    args: () => ({}),
    icon: WalletCards,
  },
  {
    tool: "agentspay_set_budget",
    label: "set budget",
    hint: "25 / 1 USDC",
    args: () => ({ daily_usd: 25, per_call_usd: 1 }),
    icon: PiggyBank,
  },
  {
    tool: "agentspay_audit_log",
    label: "audit log",
    hint: "last 5 rows",
    args: () => ({ limit: 5 }),
    icon: FileClock,
  },
  {
    tool: "agentspay_topup_info",
    label: "topup info",
    hint: "faucets",
    args: () => ({}),
    icon: Activity,
  },
];

export function SandboxTab() {
  const [lines, setLines] = useState<TerminalLine[]>([]);
  const [busy, setBusy] = useState<ToolName | null>(null);

  async function run(tool: ToolName, args: object, label: string) {
    setBusy(tool);
    setLines((prev) => [...prev, { kind: "cmd", text: label }]);
    try {
      const r = await callTool(tool, args);
      const payload = r.result?.content?.[0]?.text ?? JSON.stringify(r.result);
      setLines((prev) => [
        ...prev,
        { kind: "ok", text: `${r.latency_ms}ms | sandbox` },
        { kind: "out", text: prettyJson(payload) },
      ]);
    } catch (e) {
      setLines((prev) => [
        ...prev,
        { kind: "err", text: `error: ${(e as Error).message}` },
      ]);
    } finally {
      setBusy(null);
    }
  }

  return (
    <div className="grid gap-4 lg:grid-cols-[330px_1fr]">
      <div className="tool-panel p-4 sm:p-5">
        <p className="font-mono text-xs uppercase text-accent">Sandbox tools</p>
        <p className="mt-3 text-sm leading-6 text-fg-muted">
          Your browser tab gets an isolated SQLite ledger and keypair on the
          server. State resets after 30 minutes of inactivity.
        </p>
        <div className="mt-5 grid gap-2">
          {BUTTONS.map((button) => {
            const Icon = button.icon;
            return (
              <button
                type="button"
                key={button.tool}
                disabled={busy !== null}
                onClick={() => run(button.tool, button.args(), button.label)}
                className="grid min-h-14 grid-cols-[36px_1fr_auto] items-center gap-3 rounded-md border border-border bg-bg-panel/70 px-3 py-2 text-left transition hover:border-accent/40 hover:bg-bg-raised/60 disabled:opacity-40 disabled:hover:border-border disabled:hover:bg-bg-panel/70"
              >
                <span className="grid h-9 w-9 place-items-center rounded-md border border-border bg-bg-deep text-accent">
                  <Icon className="h-4 w-4" aria-hidden />
                </span>
                <span>
                  <span className="block text-sm font-medium text-fg">
                    {button.label}
                  </span>
                  <span className="block font-mono text-xs text-fg-muted">
                    {button.hint}
                  </span>
                </span>
                <span className="font-mono text-xs text-fg-faint">
                  {busy === button.tool ? "..." : "run"}
                </span>
              </button>
            );
          })}
        </div>
      </div>
      <div>
        <Terminal lines={lines} />
      </div>
    </div>
  );
}

function prettyJson(raw: string): string {
  try {
    return JSON.stringify(JSON.parse(raw), null, 2);
  } catch {
    return raw;
  }
}
