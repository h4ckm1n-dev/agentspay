"use client";

import { useEffect, useRef, useState } from "react";
import { fetchLatestTx, LatestTxView } from "@/lib/live-tx";
import { SolscanLink } from "./SolscanLink";

const POLL_MS = 15_000;

export function LiveTxBadge() {
  const [tx, setTx] = useState<LatestTxView | null>(null);
  const [pulse, setPulse] = useState(false);
  const lastSigRef = useRef<string | null>(null);

  useEffect(() => {
    let stop = false;
    async function tick() {
      const r = await fetchLatestTx();
      if (!stop) setTx(r);
    }
    void tick();
    const id = setInterval(tick, POLL_MS);
    return () => {
      stop = true;
      clearInterval(id);
    };
  }, []);

  useEffect(() => {
    const sig = tx?.signature ?? null;
    if (sig && sig !== lastSigRef.current) {
      // Skip pulse on the first signature we ever see — only pulse on changes.
      if (lastSigRef.current !== null) {
        setPulse(true);
        const t = setTimeout(() => setPulse(false), 1000);
        lastSigRef.current = sig;
        return () => clearTimeout(t);
      }
      lastSigRef.current = sig;
    }
    return undefined;
  }, [tx?.signature]);

  if (!tx?.signature) {
    return (
      <div className="text-xs font-mono text-fg-faint">
        No live tx in the last 24h — be the first ↓
      </div>
    );
  }
  const age = tx.age_seconds ?? 0;
  const ageStr =
    age < 60
      ? `${age}s ago`
      : age < 3600
        ? `${Math.floor(age / 60)}m ago`
        : `${Math.floor(age / 3600)}h ago`;
  return (
    <div className="text-xs font-mono text-accent flex items-center gap-3">
      <span
        className={`w-1.5 h-1.5 rounded-full bg-accent inline-block ${
          pulse ? "animate-accent-pulse" : ""
        }`}
        aria-hidden
      />
      <SolscanLink signature={tx.signature} />
      <span>· {ageStr}</span>
      {tx.amount_usdc && <span>· {tx.amount_usdc} USDC</span>}
    </div>
  );
}
