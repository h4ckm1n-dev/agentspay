"use client";

import { useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import {
  ChevronLeft,
  ChevronRight,
  ExternalLink,
  RefreshCw,
} from "lucide-react";
import {
  DevnetTransactionPage,
  DevnetTransactionView,
  fetchTransactionPage,
  formatTxAge,
} from "@/lib/live-tx";
import { shortSignature, solscanUrl } from "@/lib/proof-data";
import { cn } from "@/lib/utils";

const PAGE_SIZE = 10;
const PUBLIC_PAGE_CAP = 20;

export function ProofLedger() {
  const [page, setPage] = useState(1);
  const [data, setData] = useState<DevnetTransactionPage | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [refreshToken, setRefreshToken] = useState(0);

  useEffect(() => {
    let stopped = false;
    setLoading(true);

    async function load() {
      try {
        const next = await fetchTransactionPage(page, PAGE_SIZE);
        if (!stopped) {
          setData(next);
          setError(null);
        }
      } catch (err) {
        if (!stopped) {
          setError(err instanceof Error ? err.message : "ledger unavailable");
        }
      } finally {
        if (!stopped) setLoading(false);
      }
    }

    void load();
    return () => {
      stopped = true;
    };
  }, [page, refreshToken]);

  const totalPages = Math.min(
    data?.total_pages ?? 0,
    data?.max_pages ?? PUBLIC_PAGE_CAP,
    PUBLIC_PAGE_CAP,
  );
  const rows = data?.transactions ?? [];
  const pages = useMemo(() => buildPages(totalPages, page), [page, totalPages]);

  return (
    <div className="space-y-5">
      <div className="flex flex-col justify-between gap-3 sm:flex-row sm:items-end">
        <div>
          <div className="section-kicker">Bot transaction ledger</div>
          <p className="mt-2 max-w-2xl text-sm leading-6 text-fg-muted">
            {data
              ? `${data.total} Solscan-confirmed devnet receipts, paginated up to ${PUBLIC_PAGE_CAP} pages.`
              : "Reading Solscan-confirmed devnet receipts."}
          </p>
        </div>
        <button
          type="button"
          onClick={() => setRefreshToken((current) => current + 1)}
          className="button-secondary"
        >
          <RefreshCw className="h-4 w-4" aria-hidden />
          Refresh
        </button>
      </div>

      <div className="overflow-hidden rounded-lg border border-border bg-black/40 shadow-[0_20px_80px_rgba(0,0,0,0.24)]">
        <div className="grid grid-cols-[0.8fr_0.85fr_1.5fr] border-b border-border-subtle bg-bg-elev/75 px-4 py-3 font-mono text-xs text-fg-muted md:grid-cols-[0.55fr_0.65fr_1.35fr_1.25fr_0.75fr]">
          <span>asset</span>
          <span>amount</span>
          <span>signature</span>
          <span className="hidden md:block">endpoint</span>
          <span className="hidden md:block">age</span>
        </div>

        {loading && (
          <div className="px-4 py-8 text-sm text-fg-muted">
            Loading devnet receipts...
          </div>
        )}

        {!loading && error && (
          <div className="px-4 py-8 text-sm text-fg-muted">
            Proof ledger unavailable: {error}
          </div>
        )}

        {!loading && !error && rows.length === 0 && (
          <div className="px-4 py-8 text-sm text-fg-muted">
            No bot-made devnet transactions are recorded yet. New demo payments
            will appear here after settlement.
          </div>
        )}

        {!loading &&
          !error &&
          rows.map((row) => <LedgerRow key={row.signature} row={row} />)}
      </div>

      {totalPages > 1 && (
        <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
          <p className="font-mono text-xs text-fg-muted">
            Page {page} of {totalPages}
          </p>
          <div className="flex flex-wrap items-center gap-2">
            <PageButton
              label="Previous"
              icon={<ChevronLeft className="h-4 w-4" aria-hidden />}
              disabled={page <= 1}
              onClick={() => setPage((current) => Math.max(1, current - 1))}
            />
            {pages.map((item, index) =>
              item === "gap" ? (
                <span
                  key={`${item}-${index}`}
                  className="px-1 font-mono text-xs text-fg-faint"
                >
                  ...
                </span>
              ) : (
                <button
                  key={item}
                  type="button"
                  onClick={() => setPage(item)}
                  className={cn(
                    "grid h-9 w-9 place-items-center rounded-md border border-border font-mono text-xs transition",
                    item === page
                      ? "bg-white text-black"
                      : "text-fg-muted hover:bg-bg-elev hover:text-fg",
                  )}
                >
                  {item}
                </button>
              ),
            )}
            <PageButton
              label="Next"
              trailingIcon={<ChevronRight className="h-4 w-4" aria-hidden />}
              disabled={page >= totalPages}
              onClick={() =>
                setPage((current) => Math.min(totalPages, current + 1))
              }
            />
          </div>
        </div>
      )}
    </div>
  );
}

function LedgerRow({ row }: { readonly row: DevnetTransactionView }) {
  const href = row.explorer_url || solscanUrl(row.signature);

  return (
    <a
      href={href}
      target="_blank"
      rel="noreferrer"
      className="grid grid-cols-[0.8fr_0.85fr_1.5fr] gap-3 border-b border-border-subtle px-4 py-4 text-sm transition last:border-b-0 hover:bg-bg-elev/50 md:grid-cols-[0.55fr_0.65fr_1.35fr_1.25fr_0.75fr]"
    >
      <span className="font-mono text-fg">{row.symbol}</span>
      <span className="font-mono text-accent">{row.amount_usdc} USDC</span>
      <span>
        <span className="inline-flex items-center gap-2 font-mono text-syntax-signature">
          {shortSignature(row.signature)}
          <ExternalLink className="h-3.5 w-3.5 text-fg-faint" aria-hidden />
        </span>
        <span className="mt-1 block font-mono text-xs text-fg-muted md:hidden">
          {formatTxAge(row.age_seconds)}
        </span>
      </span>
      <span className="hidden truncate text-xs text-fg-muted md:block">
        {row.endpoint}
      </span>
      <span className="hidden font-mono text-xs text-fg-muted md:block">
        {formatTxAge(row.age_seconds)}
      </span>
    </a>
  );
}

function PageButton({
  label,
  icon,
  trailingIcon,
  disabled,
  onClick,
}: {
  readonly label: string;
  readonly icon?: ReactNode;
  readonly trailingIcon?: ReactNode;
  readonly disabled: boolean;
  readonly onClick: () => void;
}) {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      className="inline-flex h-9 items-center gap-2 rounded-md border border-border bg-bg-panel px-3 text-xs text-fg-muted transition hover:bg-bg-elev hover:text-fg disabled:cursor-not-allowed disabled:opacity-40"
    >
      {icon}
      {label}
      {trailingIcon}
    </button>
  );
}

type PageItem = number | "gap";

function buildPages(totalPages: number, currentPage: number): PageItem[] {
  if (totalPages <= 7) {
    return Array.from({ length: totalPages }, (_, i) => i + 1);
  }

  const pages = new Set<number>([
    1,
    totalPages,
    currentPage,
    Math.max(1, currentPage - 1),
    Math.min(totalPages, currentPage + 1),
  ]);
  const sorted = Array.from(pages).sort((a, b) => a - b);

  return sorted.flatMap((item, index) => {
    const previous = sorted[index - 1];
    if (previous && item - previous > 1) {
      return ["gap", item] as PageItem[];
    }
    return [item] as PageItem[];
  });
}
