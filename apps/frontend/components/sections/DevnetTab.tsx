"use client";

import { useEffect, useState } from "react";
import {
  fetchWalletStatus,
  triggerDevnet,
  DevnetWalletStatus,
  DevnetTriggerResponse,
} from "@/lib/api";
import { Terminal, TerminalLine } from "@/components/ui/Terminal";
import { SolscanLink } from "@/components/ui/SolscanLink";

export function DevnetTab() {
  const [status, setStatus] = useState<DevnetWalletStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [lines, setLines] = useState<TerminalLine[]>([]);
  const [result, setResult] = useState<DevnetTriggerResponse | null>(null);

  useEffect(() => {
    fetchWalletStatus()
      .then(setStatus)
      .catch(() => setStatus(null));
  }, []);

  async function trigger() {
    setBusy(true);
    setResult(null);
    setLines((prev) => [
      ...prev,
      { kind: "cmd", text: "agentspay_pay_url(real-quote/...)" },
    ]);
    try {
      const r = await triggerDevnet();
      setResult(r);
      setLines((prev) => [
        ...prev,
        {
          kind: "ok",
          text: `✓ ${r.latency_ms}ms · solana-devnet · ${r.amount_charged_usdc} USDC`,
        },
        {
          kind: "out",
          text:
            typeof r.body === "string"
              ? r.body
              : JSON.stringify(r.body, null, 2),
        },
      ]);
    } catch (e) {
      setLines((prev) => [
        ...prev,
        { kind: "err", text: `✗ ${(e as Error).message}` },
      ]);
    } finally {
      setBusy(false);
    }
  }

  const buttonDisabled = busy || (status !== null && !status.healthy);

  return (
    <div>
      <p className="text-fg-muted text-sm mb-4">
        One click triggers a <strong>real on-chain transaction</strong> on
        Solana devnet, signed by a server-controlled funded wallet. Rate limited
        to 1 per IP per hour to keep the faucet from draining.
      </p>

      {status && (
        <div className="text-xs text-fg-dim mb-3 font-mono">
          demo wallet: {status.pubkey.slice(0, 4)}…{status.pubkey.slice(-4)} ·{" "}
          {status.sol_balance.toFixed(3)} SOL · {status.usdc_balance.toFixed(2)}{" "}
          USDC ·{" "}
          <span className={status.healthy ? "text-accent" : "text-red-400"}>
            {status.healthy ? "healthy" : (status.message ?? "drained")}
          </span>
        </div>
      )}

      <button
        disabled={buttonDisabled}
        onClick={trigger}
        className="bg-accent text-black rounded-md px-4 py-2 text-sm font-semibold hover:opacity-90 transition disabled:opacity-40 mb-4"
      >
        {busy ? "Signing + broadcasting…" : "Trigger a real on-chain tx"}
      </button>

      {result && (
        <div className="mb-4">
          <SolscanLink signature={result.signature} />
        </div>
      )}

      <Terminal lines={lines} />
    </div>
  );
}
