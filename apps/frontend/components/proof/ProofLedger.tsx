"use client";

import { useEffect, useMemo, useState } from "react";
import { fetchLatestTx, LatestTxView } from "@/lib/live-tx";
import {
  PROOF_RECORDS,
  ProofRecord,
  shortSignature,
  solscanUrl,
} from "@/lib/proof-data";

interface LedgerRow extends ProofRecord {
  readonly age?: string;
  readonly live?: boolean;
}

export function ProofLedger() {
  const [latest, setLatest] = useState<LatestTxView | null>(null);

  useEffect(() => {
    let stopped = false;
    async function tick() {
      const tx = await fetchLatestTx();
      if (!stopped) {
        setLatest(tx);
      }
    }
    void tick();
    const id = setInterval(tick, 15_000);
    return () => {
      stopped = true;
      clearInterval(id);
    };
  }, []);

  const rows = useMemo<LedgerRow[]>(() => {
    const base = [...PROOF_RECORDS];
    const latestSig = latest?.signature;
    if (
      latestSig &&
      !base.some((record) => record.signature === latestSig)
    ) {
      return [
        {
          symbol: "LIVE",
          amountUsdc: latest.amount_usdc ?? "0.10",
          signature: latestSig,
          context: "Most recent browser-triggered devnet settlement",
          age: formatAge(latest.age_seconds),
          live: true,
        },
        ...base,
      ];
    }
    return base;
  }, [latest]);

  return (
    <div className="overflow-hidden rounded-md border border-border">
      <div className="grid grid-cols-[0.8fr_0.9fr_1.7fr] border-b border-border-subtle bg-bg-elev px-4 py-3 font-mono text-xs text-fg-muted md:grid-cols-[0.7fr_0.7fr_2fr_1.2fr_1.2fr]">
        <span>asset</span>
        <span>amount</span>
        <span>signature</span>
        <span className="hidden md:block">payer</span>
        <span className="hidden md:block">payee</span>
      </div>
      {rows.map((row) => (
        <a
          key={row.signature}
          href={solscanUrl(row.signature)}
          className="grid grid-cols-[0.8fr_0.9fr_1.7fr] gap-3 border-b border-border-subtle px-4 py-4 text-sm transition last:border-b-0 hover:bg-bg-elev/60 md:grid-cols-[0.7fr_0.7fr_2fr_1.2fr_1.2fr]"
        >
          <span className="font-mono text-fg">
            {row.symbol}
            {row.live && <span className="ml-2 text-accent">live</span>}
          </span>
          <span className="font-mono text-accent">{row.amountUsdc} USDC</span>
          <span>
            <span className="block font-mono text-syntax-signature">
              {shortSignature(row.signature)}
            </span>
            <span className="mt-1 block text-xs text-fg-muted">
              {row.age ? `${row.context} - ${row.age}` : row.context}
            </span>
          </span>
          <span className="hidden truncate font-mono text-xs text-syntax-pubkey md:block">
            {row.payer ?? "server wallet"}
          </span>
          <span className="hidden truncate font-mono text-xs text-syntax-pubkey md:block">
            {row.payee ?? "demo provider"}
          </span>
        </a>
      ))}
    </div>
  );
}

function formatAge(ageSeconds: number | null | undefined): string | undefined {
  if (ageSeconds === null || ageSeconds === undefined) {
    return undefined;
  }
  if (ageSeconds < 60) {
    return `${ageSeconds}s ago`;
  }
  if (ageSeconds < 3600) {
    return `${Math.floor(ageSeconds / 60)}m ago`;
  }
  return `${Math.floor(ageSeconds / 3600)}h ago`;
}
