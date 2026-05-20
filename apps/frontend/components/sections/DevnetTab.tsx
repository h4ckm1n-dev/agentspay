"use client";

import { useEffect, useState } from "react";
import {
  CircleDollarSign,
  ReceiptText,
  Send,
  ServerCog,
  WalletCards,
} from "lucide-react";
import {
  requestDevnetPayment,
  fetchWalletStatus,
  triggerDevnet,
  DevnetPaymentRequestResponse,
  DevnetWalletStatus,
  DevnetTriggerResponse,
} from "@/lib/api";
import { Terminal, TerminalLine } from "@/components/ui/Terminal";
import { SolscanLink } from "@/components/ui/SolscanLink";

const SYMBOLS = ["AAPL", "MSFT", "GOOG", "NVDA", "AMZN"] as const;

export function DevnetTab() {
  const [symbol, setSymbol] = useState<(typeof SYMBOLS)[number]>("GOOG");
  const [status, setStatus] = useState<DevnetWalletStatus | null>(null);
  const [busy, setBusy] = useState<"request" | "pay" | null>(null);
  const [lines, setLines] = useState<TerminalLine[]>([]);
  const [paymentRequest, setPaymentRequest] =
    useState<DevnetPaymentRequestResponse | null>(null);
  const [result, setResult] = useState<DevnetTriggerResponse | null>(null);

  useEffect(() => {
    fetchWalletStatus()
      .then(setStatus)
      .catch(() => setStatus(null));
  }, []);

  async function askForPayment() {
    setBusy("request");
    setResult(null);
    setLines((prev) => [
      ...prev,
      { kind: "cmd", text: `GET /real-quote/${symbol}` },
    ]);
    try {
      const response = await requestDevnetPayment(symbol);
      setPaymentRequest(response);
      setLines((prev) => [
        ...prev,
        {
          kind: "ok",
          text: `payment required | ${response.status} | ${
            response.amount_usdc ?? "?"
          } USDC`,
        },
        { kind: "out", text: JSON.stringify(response.body, null, 2) },
      ]);
    } catch (error) {
      setLines((prev) => [
        ...prev,
        { kind: "err", text: `error: ${(error as Error).message}` },
      ]);
    } finally {
      setBusy(null);
    }
  }

  async function pay() {
    setBusy("pay");
    setResult(null);
    setLines((prev) => [
      ...prev,
      { kind: "cmd", text: `agentspay_pay_url(real-quote/${symbol})` },
    ]);
    try {
      const response = await triggerDevnet(symbol);
      setResult(response);
      setPaymentRequest(null);
      setLines((prev) => [
        ...prev,
        {
          kind: "ok",
          text: `${response.latency_ms}ms | solana-devnet | ${response.amount_charged_usdc} USDC`,
        },
        {
          kind: "out",
          text:
            typeof response.body === "string"
              ? response.body
              : JSON.stringify(response.body, null, 2),
        },
      ]);
    } catch (error) {
      setLines((prev) => [
        ...prev,
        { kind: "err", text: `error: ${(error as Error).message}` },
      ]);
    } finally {
      setBusy(null);
    }
  }

  const walletDisabled = status !== null && !status.healthy;

  return (
    <div className="grid gap-4 lg:grid-cols-[380px_1fr]">
      <div className="space-y-4">
        <div className="tool-panel p-4 sm:p-5">
          <div className="flex items-start gap-3">
            <span className="grid h-10 w-10 shrink-0 place-items-center rounded-md border border-accent/20 bg-accent/10 text-accent">
              <ServerCog className="h-5 w-5" aria-hidden />
            </span>
            <div>
              <p className="font-mono text-xs uppercase text-accent">
                Devnet payment
              </p>
              <p className="mt-2 text-sm leading-6 text-fg-muted">
                Ask for an x402 quote, then pay it with a real on-chain Solana
                devnet USDC transfer. The pay action is limited to one per IP
                per hour.
              </p>
            </div>
          </div>

          {status && (
            <div className="mt-5 rounded-md border border-border bg-bg-deep p-3 font-mono text-xs text-fg-muted">
              <div className="mb-2 flex items-center gap-2 text-fg">
                <WalletCards className="h-4 w-4 text-accent" aria-hidden />
                demo wallet
              </div>
              <p>
                {status.pubkey.slice(0, 4)}...{status.pubkey.slice(-4)}
              </p>
              <p className="mt-2">
                {status.sol_balance.toFixed(3)} SOL /{" "}
                {status.usdc_balance.toFixed(2)} USDC
              </p>
              <p
                className={`mt-2 ${
                  status.healthy ? "text-accent" : "text-red-400"
                }`}
              >
                {status.healthy ? "healthy" : (status.message ?? "drained")}
              </p>
            </div>
          )}
        </div>

        <div className="tool-panel p-4 sm:p-5">
          <label className="block font-mono text-xs uppercase text-fg-muted">
            Asset
            <select
              value={symbol}
              onChange={(event) => {
                setSymbol(event.target.value as (typeof SYMBOLS)[number]);
                setPaymentRequest(null);
                setResult(null);
              }}
              className="mt-2 w-full rounded-md border border-border bg-bg px-3 py-2.5 text-sm text-fg"
            >
              {SYMBOLS.map((item) => (
                <option key={item} value={item}>
                  {item}
                </option>
              ))}
            </select>
          </label>

          <div className="mt-4 grid gap-2">
            <button
              type="button"
              disabled={busy !== null}
              onClick={askForPayment}
              className="button-secondary w-full disabled:opacity-40"
            >
              <ReceiptText className="h-4 w-4" aria-hidden />
              {busy === "request" ? "Asking..." : "Ask for payment"}
            </button>

            <button
              type="button"
              disabled={busy !== null || walletDisabled || paymentRequest === null}
              onClick={pay}
              className="inline-flex min-h-10 items-center justify-center gap-2 rounded-md bg-accent px-4 py-2.5 text-sm font-semibold text-black shadow-[0_0_24px_rgba(20,184,166,0.24)] transition hover:bg-accent/90 disabled:opacity-40 disabled:shadow-none"
            >
              <Send className="h-4 w-4" aria-hidden />
              {busy === "pay" ? "Signing..." : "Pay this request"}
            </button>
          </div>
        </div>

        {paymentRequest && (
          <div className="tool-panel p-4 text-xs">
            <div className="mb-4 flex items-center gap-2 font-mono uppercase text-accent">
              <CircleDollarSign className="h-4 w-4" aria-hidden />
              Active quote
            </div>
            <div className="grid gap-3">
              <Fact label="asset" value={paymentRequest.symbol} />
              <Fact
                label="amount"
                value={`${paymentRequest.amount_usdc ?? "?"} USDC`}
              />
              <Fact
                label="network"
                value={paymentRequest.network ?? "unknown"}
              />
              <Fact label="payTo" value={paymentRequest.pay_to ?? "missing"} />
            </div>
          </div>
        )}

        {result && <SolscanLink signature={result.signature} />}
      </div>

      <Terminal lines={lines} />
    </div>
  );
}

function Fact({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0 rounded-md border border-border bg-bg-deep px-3 py-2">
      <p className="font-mono uppercase text-fg-faint">{label}</p>
      <p className="mt-1 truncate font-mono text-fg">{value}</p>
    </div>
  );
}
