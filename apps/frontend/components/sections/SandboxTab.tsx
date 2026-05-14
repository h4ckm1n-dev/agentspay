"use client";

import { useState } from "react";
import { callTool, ToolName } from "@/lib/api";
import { Terminal, TerminalLine } from "@/components/ui/Terminal";

interface ToolButton {
  tool: ToolName;
  label: string;
  args: () => object;
}

const BUTTONS: ToolButton[] = [
  { tool: "agentspay_balance", label: "balance()", args: () => ({}) },
  {
    tool: "agentspay_set_budget",
    label: "set_budget(daily=25, per_call=1)",
    args: () => ({ daily_usd: 25, per_call_usd: 1 }),
  },
  {
    tool: "agentspay_audit_log",
    label: "audit_log(limit=5)",
    args: () => ({ limit: 5 }),
  },
  { tool: "agentspay_topup_info", label: "topup_info()", args: () => ({}) },
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
        { kind: "ok", text: `✓ ${r.latency_ms}ms · sandbox` },
        { kind: "out", text: prettyJson(payload) },
      ]);
    } catch (e) {
      setLines((prev) => [
        ...prev,
        { kind: "err", text: `✗ ${(e as Error).message}` },
      ]);
    } finally {
      setBusy(null);
    }
  }

  return (
    <div>
      <p className="text-fg-muted text-sm mb-4">
        Call any of the 4 read/write tools below. Your tab gets an isolated
        SQLite ledger + keypair on the server. State resets after 30 minutes of
        inactivity.
      </p>
      <div className="flex flex-wrap gap-2 mb-4">
        {BUTTONS.map((b) => (
          <button
            key={b.tool}
            disabled={busy !== null}
            onClick={() => run(b.tool, b.args(), b.label)}
            className="bg-bg-elev border border-border rounded-md px-3 py-1.5 text-xs font-mono hover:bg-border-subtle transition disabled:opacity-40"
          >
            {busy === b.tool ? "…" : b.label}
          </button>
        ))}
      </div>
      <Terminal lines={lines} />
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
