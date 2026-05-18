export interface LatestTxView {
  signature: string | null;
  amount_usdc?: string;
  explorer_url?: string;
  age_seconds?: number | null;
}

export interface DevnetTransactionView {
  signature: string;
  amount_usdc: string;
  explorer_url: string;
  endpoint: string;
  symbol: string;
  status: string;
  agent_id: string;
  payment_id: string;
  created_at: string;
  age_seconds?: number | null;
}

export interface DevnetTransactionPage {
  page: number;
  page_size: number;
  total: number;
  total_pages: number;
  max_pages: number;
  transactions: DevnetTransactionView[];
}

export async function fetchLatestTx(): Promise<LatestTxView> {
  const r = await fetch("/api/stats/latest-tx", { cache: "no-store" });
  if (!r.ok) return { signature: null };
  return (await r.json()) as LatestTxView;
}

export async function fetchTransactionPage(
  page = 1,
  pageSize = 10,
): Promise<DevnetTransactionPage> {
  const params = new URLSearchParams({
    page: page.toString(),
    page_size: pageSize.toString(),
  });
  const r = await fetch(`/api/stats/transactions?${params.toString()}`, {
    cache: "no-store",
  });
  if (!r.ok) {
    throw new Error(`transactions unavailable: ${r.status}`);
  }
  return (await r.json()) as DevnetTransactionPage;
}

export async function fetchRecentTransactions(
  limit = 12,
): Promise<DevnetTransactionView[]> {
  const params = new URLSearchParams({ limit: limit.toString() });
  const r = await fetch(`/api/stats/transactions?${params.toString()}`, {
    cache: "no-store",
  });
  if (!r.ok) return [];
  const page = (await r.json()) as DevnetTransactionPage;
  return page.transactions;
}

export function formatTxAge(ageSeconds?: number | null): string {
  if (ageSeconds === null || ageSeconds === undefined) return "just now";
  if (ageSeconds < 60) return `${ageSeconds}s ago`;
  if (ageSeconds < 3600) return `${Math.floor(ageSeconds / 60)}m ago`;
  if (ageSeconds < 86_400) return `${Math.floor(ageSeconds / 3600)}h ago`;
  return `${Math.floor(ageSeconds / 86_400)}d ago`;
}
