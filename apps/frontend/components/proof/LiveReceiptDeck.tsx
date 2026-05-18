"use client";

import { useEffect, useMemo, useState } from "react";
import {
  ArrowRight,
  ExternalLink,
  Radio,
  ReceiptText,
  RefreshCw,
} from "lucide-react";
import {
  DevnetTransactionView,
  fetchRecentTransactions,
  formatTxAge,
} from "@/lib/live-tx";
import { shortSignature, solscanUrl } from "@/lib/proof-data";
import { cn } from "@/lib/utils";

const POLL_MS = 15_000;
const ROTATE_MS = 4_000;

type ReceiptVariant = "hero" | "strip";

interface LiveReceiptDeckProps {
  readonly variant?: ReceiptVariant;
  readonly limit?: number;
  readonly visibleCount?: number;
  readonly className?: string;
}

export function LiveReceiptDeck({
  variant = "strip",
  limit = 12,
  visibleCount = 3,
  className,
}: LiveReceiptDeckProps) {
  const [transactions, setTransactions] = useState<DevnetTransactionView[]>([]);
  const [offset, setOffset] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let stopped = false;

    async function load() {
      try {
        const rows = await fetchRecentTransactions(limit);
        if (!stopped) {
          setTransactions(rows);
          setError(null);
        }
      } catch (err) {
        if (!stopped) {
          setError(
            err instanceof Error ? err.message : "proof feed unavailable",
          );
        }
      } finally {
        if (!stopped) setLoading(false);
      }
    }

    void load();
    const id = setInterval(load, POLL_MS);
    return () => {
      stopped = true;
      clearInterval(id);
    };
  }, [limit]);

  useEffect(() => {
    if (transactions.length <= visibleCount) return undefined;
    const id = setInterval(() => {
      setOffset((current) => (current + visibleCount) % transactions.length);
    }, ROTATE_MS);
    return () => clearInterval(id);
  }, [transactions.length, visibleCount]);

  const visible = useMemo(
    () => buildVisibleTransactions(transactions, offset, visibleCount),
    [offset, transactions, visibleCount],
  );

  if (loading) {
    return (
      <div className={cn("grid gap-3 md:grid-cols-3", className)}>
        {Array.from({ length: visibleCount }).map((_, index) => (
          <div
            key={index}
            className="min-h-[156px] animate-pulse rounded-lg border border-border bg-bg-panel/60 p-4"
          >
            <div className="h-3 w-24 rounded bg-bg-raised" />
            <div className="mt-8 h-4 w-36 rounded bg-bg-raised" />
            <div className="mt-6 h-3 w-full rounded bg-bg-raised" />
          </div>
        ))}
      </div>
    );
  }

  if (error || visible.length === 0) {
    return (
      <div
        className={cn(
          "rounded-lg border border-dashed border-border bg-bg-panel/50 p-5",
          className,
        )}
      >
        <div className="flex items-center gap-2 font-mono text-xs uppercase text-fg-muted">
          <Radio className="h-3.5 w-3.5" aria-hidden />
          Devnet proof feed
        </div>
        <p className="mt-3 max-w-2xl text-sm leading-relaxed text-fg-muted">
          No bot-made devnet receipts are available yet. Trigger a demo payment
          and the newest Solscan-confirmed rows will appear here automatically.
        </p>
      </div>
    );
  }

  const rotating = transactions.length > visibleCount;

  return (
    <div className={cn("space-y-4", className)}>
      {variant === "hero" && (
        <ReceiptDeckHeader count={transactions.length} rotating={rotating} />
      )}

      <div
        className={cn(
          "grid gap-3",
          variant === "strip" ? "md:grid-cols-3" : "grid-cols-1",
        )}
      >
        {visible.map((tx) => (
          <ReceiptCard
            key={tx.signature}
            tx={tx}
            compact={variant === "hero"}
          />
        ))}
      </div>
    </div>
  );
}

export function LiveReceiptTicker({
  limit = 12,
  className,
}: {
  readonly limit?: number;
  readonly className?: string;
}) {
  const [transactions, setTransactions] = useState<DevnetTransactionView[]>([]);
  const [offset, setOffset] = useState(0);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let stopped = false;

    async function load() {
      try {
        const rows = await fetchRecentTransactions(limit);
        if (!stopped) {
          setTransactions(rows);
          setOffset(0);
        }
      } finally {
        if (!stopped) {
          setLoading(false);
        }
      }
    }

    void load();
    const id = setInterval(load, POLL_MS);
    return () => {
      stopped = true;
      clearInterval(id);
    };
  }, [limit]);

  useEffect(() => {
    if (transactions.length <= 1) return undefined;
    const id = setInterval(() => {
      setOffset((current) => (current + 1) % transactions.length);
    }, ROTATE_MS);
    return () => clearInterval(id);
  }, [transactions.length]);

  const tx = transactions[offset];

  return (
    <div className={cn("tool-panel overflow-hidden", className)}>
      <div className="flex flex-col gap-3 border-b border-border-subtle bg-bg-elev/70 px-4 py-3 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex items-center gap-2">
          <span className="grid h-8 w-8 place-items-center rounded-md border border-accent/20 bg-accent/10 text-accent">
            <ReceiptText className="h-4 w-4" aria-hidden />
          </span>
          <div>
            <p className="font-mono text-xs uppercase text-accent">
              Live transaction card
            </p>
            <p className="text-xs text-fg-muted">
              Solscan-confirmed rows from the devnet ledger
            </p>
          </div>
        </div>
        <a
          href="/proof"
          className="inline-flex items-center gap-2 font-mono text-xs text-fg-muted transition hover:text-fg"
        >
          full ledger
          <ArrowRight className="h-3.5 w-3.5" aria-hidden />
        </a>
      </div>

      {loading ? (
        <div className="grid gap-3 p-4 sm:grid-cols-[1fr_auto] sm:items-center">
          <div className="space-y-3">
            <div className="h-3 w-40 animate-pulse rounded bg-bg-raised" />
            <div className="h-5 w-64 max-w-full animate-pulse rounded bg-bg-raised" />
          </div>
          <div className="h-10 w-28 animate-pulse rounded-md bg-bg-raised" />
        </div>
      ) : tx ? (
        <HeroReceipt tx={tx} rotating={transactions.length > 1} />
      ) : (
        <div className="p-4">
          <p className="text-sm leading-6 text-fg-muted">
            Live receipts will rotate here as bot traffic lands. Run the devnet
            demo to create the first visible settlement.
          </p>
        </div>
      )}
    </div>
  );
}

function HeroReceipt({
  tx,
  rotating,
}: {
  readonly tx: DevnetTransactionView;
  readonly rotating: boolean;
}) {
  const href = tx.explorer_url || solscanUrl(tx.signature);

  return (
    <a
      href={href}
      target="_blank"
      rel="noreferrer"
      className="group block p-4 transition hover:bg-bg-elev/40"
    >
      <div className="grid gap-4 sm:grid-cols-[1fr_auto] sm:items-end">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <span className="inline-flex items-center gap-2 rounded-md border border-accent/25 bg-accent/10 px-2 py-1 font-mono text-xs text-accent">
              <span className="h-1.5 w-1.5 rounded-full bg-accent shadow-[0_0_16px_rgba(20,184,166,0.8)]" />
              {rotating ? "rotating" : "latest"}
            </span>
            <span className="rounded-md border border-border bg-bg-deep px-2 py-1 font-mono text-xs text-fg-muted">
              {formatTxAge(tx.age_seconds)}
            </span>
          </div>
          <div className="mt-4 flex flex-wrap items-baseline gap-x-3 gap-y-2">
            <span className="font-mono text-2xl text-fg">{tx.symbol}</span>
            <span className="font-mono text-xl text-accent">
              {tx.amount_usdc} USDC
            </span>
          </div>
          <p className="mt-3 min-w-0 truncate font-mono text-sm text-syntax-signature">
            {shortSignature(tx.signature)}
          </p>
          <p className="mt-2 min-w-0 truncate text-xs text-fg-muted">
            {tx.endpoint}
          </p>
        </div>

        <span className="inline-flex h-10 items-center justify-center gap-2 rounded-md border border-border bg-bg-panel px-3 font-mono text-xs text-fg-muted transition group-hover:border-accent/50 group-hover:text-fg">
          Solscan
          <ExternalLink className="h-3.5 w-3.5" aria-hidden />
        </span>
      </div>
    </a>
  );
}

function ReceiptDeckHeader({
  count,
  rotating,
}: {
  readonly count: number;
  readonly rotating: boolean;
}) {
  return (
    <div className="flex items-center justify-between gap-3">
      <div>
        <p className="font-mono text-xs uppercase text-accent">
          Live devnet receipts
        </p>
        <p className="mt-1 text-xs text-fg-muted">
          {rotating
            ? `Rotating through ${count} recent bot payments`
            : "Newest Solscan-confirmed bot payments"}
        </p>
      </div>
      {rotating && (
        <RefreshCw className="h-4 w-4 animate-spin text-accent" aria-hidden />
      )}
    </div>
  );
}

function ReceiptCard({
  tx,
  compact,
}: {
  readonly tx: DevnetTransactionView;
  readonly compact: boolean;
}) {
  const href = tx.explorer_url || solscanUrl(tx.signature);

  return (
    <a
      href={href}
      target="_blank"
      rel="noreferrer"
      className={cn(
        "group block min-h-[156px] rounded-lg border border-border bg-bg-panel/70 transition hover:border-accent/60 hover:bg-bg-raised/50",
        compact ? "p-3" : "p-4 shadow-[0_18px_70px_rgba(0,0,0,0.22)]",
      )}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <span className="h-2 w-2 shrink-0 rounded-full bg-accent shadow-[0_0_18px_rgba(20,184,166,0.8)]" />
            <span className="font-mono text-sm text-fg">{tx.symbol}</span>
          </div>
          <p className="mt-2 truncate font-mono text-xs text-syntax-signature">
            {shortSignature(tx.signature)}
          </p>
        </div>
        <ExternalLink
          className="h-4 w-4 shrink-0 text-fg-faint transition group-hover:text-accent"
          aria-hidden
        />
      </div>
      <div className="mt-5 flex items-end justify-between gap-3 border-t border-border-subtle pt-3">
        <span className="font-mono text-sm text-accent">
          {tx.amount_usdc} USDC
        </span>
        <span className="font-mono text-xs text-fg-muted">
          {formatTxAge(tx.age_seconds)}
        </span>
      </div>
      {!compact && (
        <p className="mt-3 truncate text-xs text-fg-muted">{tx.endpoint}</p>
      )}
    </a>
  );
}

function buildVisibleTransactions(
  transactions: DevnetTransactionView[],
  offset: number,
  visibleCount: number,
): DevnetTransactionView[] {
  if (transactions.length <= visibleCount) return transactions;

  const rows: DevnetTransactionView[] = [];
  for (let i = 0; i < visibleCount; i += 1) {
    const tx = transactions[(offset + i) % transactions.length];
    if (tx) rows.push(tx);
  }
  return rows;
}
