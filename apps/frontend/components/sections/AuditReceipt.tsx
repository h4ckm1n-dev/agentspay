"use client";

import { useEffect, useRef, useState } from "react";
import { motion, useInView, useReducedMotion } from "motion/react";
import { Ban, ShieldCheck, Sigma, Wallet } from "lucide-react";

const CAP = 1.0;
const TARGET = 2.4;
const FILL_MS = 1500;
const HOLD_MS = 3200;

/**
 * The hero artifact. A payment request counts up, blows past the per-call cap,
 * and gets stamped REJECTED before any signature exists — the guardrail firing.
 * The verdict slot is always filled (evaluating → rejected) so the card keeps a
 * constant height and never looks hollow mid-animation. Renders the final
 * blocked state for reduced-motion users.
 */
export function AuditReceipt() {
  const ref = useRef<HTMLDivElement>(null);
  const inView = useInView(ref, { margin: "-80px" });
  const reduceMotion = useReducedMotion();
  const [amount, setAmount] = useState(reduceMotion ? TARGET : 0);
  const [rejected, setRejected] = useState(Boolean(reduceMotion));

  useEffect(() => {
    if (reduceMotion || !inView) return;
    let raf = 0;
    let timeout: ReturnType<typeof setTimeout>;
    let cancelled = false;

    const run = () => {
      if (cancelled) return;
      setRejected(false);
      setAmount(0);
      const start = performance.now();
      const tick = (now: number) => {
        if (cancelled) return;
        const t = Math.min((now - start) / FILL_MS, 1);
        const eased = 1 - Math.pow(1 - t, 2);
        setAmount(Number((eased * TARGET).toFixed(2)));
        if (t < 1) {
          raf = requestAnimationFrame(tick);
        } else {
          setRejected(true);
          timeout = setTimeout(run, HOLD_MS);
        }
      };
      raf = requestAnimationFrame(tick);
    };

    run();
    return () => {
      cancelled = true;
      cancelAnimationFrame(raf);
      clearTimeout(timeout);
    };
  }, [inView, reduceMotion]);

  const fillPct = Math.min(amount / TARGET, 1) * 100;
  const capPct = (CAP / TARGET) * 100;
  const over = amount > CAP;

  return (
    <div
      ref={ref}
      className="gradient-hairline glow-emerald flex min-h-[19rem] flex-col overflow-hidden rounded-xl2 border border-border-strong bg-gradient-to-b from-bg-panel to-bg-deep"
    >
      <div className="flex items-center justify-between border-b border-border bg-white/[0.015] px-4 py-3 font-mono text-xs text-fg-muted">
        <span className="flex items-center gap-2">
          <Wallet className="h-3.5 w-3.5 text-accent" aria-hidden />
          agentspay · pre-sign audit
        </span>
        <span className="flex gap-1.5" aria-hidden>
          <i className="h-2.5 w-2.5 rounded-full bg-syntax-null" />
          <i className="h-2.5 w-2.5 rounded-full bg-accent-gold" />
          <i className="h-2.5 w-2.5 rounded-full bg-accent-mint" />
        </span>
      </div>

      <div className="flex flex-1 flex-col px-4 py-5 font-mono text-[13px] leading-relaxed">
        <div className="space-y-2">
          <Row k="tool" v="agentspay_pay_url" className="text-syntax-usdc" />
          <Row
            k="amount_usdc"
            v={amount.toFixed(2)}
            className={over ? "font-bold text-gate" : "text-accent-gold"}
          />
          <Row k="per_call_cap" v="1.00" className="text-accent-gold" />
        </div>

        {/* budget meter: emerald up to the cap, amber over it */}
        <div className="mt-3 flex items-center gap-3">
          <div className="relative h-2 flex-1 overflow-hidden rounded-full border border-border bg-bg-deep">
            <div
              className="absolute inset-y-0 left-0 bg-accent"
              style={{ width: `${Math.min(fillPct, capPct)}%` }}
            />
            {over && (
              <div
                className="absolute inset-y-0 bg-gate"
                style={{ left: `${capPct}%`, width: `${fillPct - capPct}%` }}
              />
            )}
            <div
              className="absolute inset-y-0 w-px bg-fg/50"
              style={{ left: `${capPct}%` }}
            />
          </div>
          <span className="shrink-0 text-[11px] text-fg-faint">cap 1.00</span>
        </div>

        {/* verdict slot — pinned to the bottom of the body, always filled */}
        <div className="mt-auto pt-4">
          {rejected ? (
            <motion.div
              initial={reduceMotion ? false : { opacity: 0, scale: 1.12 }}
              animate={{ opacity: 1, scale: 1 }}
              transition={{ type: "spring", stiffness: 340, damping: 18 }}
              className="flex items-center gap-3 rounded-lg border border-gate-line bg-gate-soft px-3.5 py-3 text-gate"
            >
              <span className="grid h-5 w-5 place-items-center rounded-md bg-gate text-bg-deep">
                <Ban className="h-3.5 w-3.5" aria-hidden />
              </span>
              <span className="font-bold tracking-wide">
                REJECTED — exceeds per-call cap
              </span>
            </motion.div>
          ) : (
            <div className="flex items-center gap-3 rounded-lg border border-border bg-white/[0.02] px-3.5 py-3 text-fg-muted">
              <span className="h-2 w-2 animate-pulse rounded-full bg-accent" />
              <span>evaluating per-call policy…</span>
            </div>
          )}
        </div>
      </div>

      <div className="flex flex-wrap gap-x-5 gap-y-1 border-t border-border px-4 py-3 font-mono text-[11px] text-fg-faint">
        <span className="flex items-center gap-1.5 text-accent">
          <ShieldCheck className="h-3 w-3" aria-hidden /> ssrf_guard pass
        </span>
        <span className="text-accent">asset USDC</span>
        <span className="text-accent">decimals 6</span>
        <span className="flex items-center gap-1.5">
          <Sigma className="h-3 w-3" aria-hidden /> no signature emitted
        </span>
      </div>
    </div>
  );
}

function Row({
  k,
  v,
  className,
}: {
  k: string;
  v: string;
  className?: string;
}) {
  return (
    <div className="grid grid-cols-[150px_1fr] gap-3">
      <span className="text-fg-faint">{k}</span>
      <span className={className}>{v}</span>
    </div>
  );
}
