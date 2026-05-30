const ITEMS = [
  "Claude Code",
  "Cursor",
  "Cline",
  "Zed",
  "x402",
  "Solana devnet",
  "USDC",
  "SPL transfer_checked",
] as const;

/**
 * Scrolling "works with" strip. CSS-only marquee (the list is duplicated for a
 * seamless loop); honors prefers-reduced-motion via the global media query.
 */
export function HostMarquee() {
  return (
    <div className="relative overflow-hidden border-b border-border-subtle bg-black/20 py-5">
      <div className="section-shell mb-3">
        <span className="font-mono text-xs uppercase tracking-[0.14em] text-fg-faint">
          Runs in your stack
        </span>
      </div>
      <div className="relative [mask-image:linear-gradient(90deg,transparent,#000_10%,#000_90%,transparent)]">
        <div className="flex w-max animate-marquee items-center gap-10 pr-10">
          {[...ITEMS, ...ITEMS].map((item, i) => (
            <span
              key={i}
              className="flex shrink-0 items-center gap-3 font-mono text-sm text-fg-muted"
            >
              <span className="h-1.5 w-1.5 rounded-full bg-accent/70" />
              {item}
            </span>
          ))}
        </div>
      </div>
    </div>
  );
}
