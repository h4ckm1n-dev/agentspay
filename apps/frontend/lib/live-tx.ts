export interface LatestTxView {
  signature: string | null;
  amount_usdc?: string;
  explorer_url?: string;
  age_seconds?: number | null;
}

export async function fetchLatestTx(): Promise<LatestTxView> {
  const r = await fetch("/api/stats/latest-tx", { cache: "no-store" });
  if (!r.ok) return { signature: null };
  return (await r.json()) as LatestTxView;
}
