# Design System — AgentsPay (v2 "terminal-luxe")

## Product Context
- **What this is:** A local MCP server that gives AI coding agents a budget-capped USDC wallet for x402-priced APIs.
- **Who it's for:** Senior developers and infra engineers who are about to let an AI agent spend money.
- **Space/industry:** Developer infrastructure / agent tooling / crypto payments (Solana devnet).
- **Project type:** Marketing + product site (Next.js 15 App Router): landing, `/demo`, `/docs`, `/proof`.
- **The one thing to remember:** *The agent can spend, but it cannot drain the wallet.* Every design decision serves this.

## Aesthetic Direction
- **Direction:** Terminal-luxe — Linear crossed with a hardware wallet. Near-black, premium, restrained; type does the work.
- **Decoration level:** Intentional. A subtle emerald aurora at the top of heroes; a fine grid texture that fades below the fold (the "auditable surface"). No purple gradients, no icon-circle grids, no centered-everything.
- **The differentiator:** Every peer (Linear, Vercel, Stripe, x402) sells *capability*. AgentsPay sells *restraint*, so the hero artifact shows the guardrail **firing** — a payment rejected for exceeding the per-call cap, `no signature emitted` — not a happy-path success. The denial is the memorable image.
- **Reference sites:** linear.app (restraint, quiet display type, code-as-proof), vercel.com (metric strip, code blocks), x402.org (big-number stat band, oversized wordmark), stripe.com (the artifact as hero).

## Typography
- **Display/Hero:** Satoshi (900/700) — self-hosted via `next/font/local` (`app/fonts/`, `--font-display`). Carries the brand claim.
- **Body / UI:** Geist — `next/font/google` (`--font-sans`).
- **Data / receipts / code:** JetBrains Mono — `next/font/google` (`--font-mono`). What you see on the page matches the ledger.
- **Base rule:** `h1, h2, h3` use the display family with `-0.03em` tracking (`app/globals.css`).
- **Hero scale:** clamp from `text-5xl` to `text-7xl`, `font-black`, `leading-[0.95]`, `tracking-tightest`.

## Color (Tailwind tokens — `tailwind.config.ts`)
- **Background:** `bg` `#070708`, `bg-panel` `#101014`, `bg-elev` `#0d0d10`, `bg-deep` `#000000`.
- **Borders:** `border` `#27272a`, `border-subtle` `#1c1c20`, `border-strong` `#33333a`.
- **Foreground:** `fg` `#fafafa`, `fg-muted` `#a1a1aa`, `fg-dim` `#71717a`, `fg-faint` `#5b5b63`.
- **Accent (the product):** `accent` `#10b981`, `accent-mint` `#34d399`, used sparingly.
- **Gate (reserved):** `gate` `#fbbf24` + `gate-soft` / `gate-line`. Used for **one thing only**: the moment a guardrail blocks a payment. Never decorative.
- **Syntax tokens:** preserved from v1 (Solana/fintech-aware: signature, pubkey, usdc, timestamp, etc.).

## Spacing & Layout
- **Container:** `.section-shell` — `max-w-7xl`, responsive padding.
- **Density:** comfortable. Sections at `py-16 sm:py-20`; hero at `py-16 lg:py-24`.
- **Radius:** `rounded-xl2` (1.125rem) is the panel/button default; full for pills.
- **Hero grid:** `lg:grid-cols-[1.05fr_0.95fr]` — claim left, audit-receipt artifact right.

## Motion (`motion` package + `components/ui/Reveal.tsx`)
- **Approach:** Intentional. Scroll-reveal on section entrance (`fade-up`, `whileInView`, `once`, `-80px` margin), staggered for card grids.
- **Easing:** `cubic-bezier(0.22, 1, 0.36, 1)`; duration ~0.55s.
- **Accessibility:** `Reveal` honors `prefers-reduced-motion` — those users get content immediately, never hidden behind a JS animation. Global `@media (prefers-reduced-motion)` zeroes CSS animations/transitions.
- **Ambient:** `aurora-drift` (hero glow), `marquee` (live receipt ticker), `accent-pulse`.

## Key Components
- `Hero` — claim + `AuditReceipt` (the rejected-payment artifact) + metric pills + stat band.
- `Reveal` — client motion island wrapping server-rendered section content.
- Shared classes (`globals.css` `@layer components`): `.aurora`, `.page-hero`, `.page-band`, `.section-kicker`, `.eyebrow`, `.metric-pill`, `.tool-panel`, `.quiet-panel`, `.button-primary` (emerald), `.button-secondary`, `.code-chip`.

## Decisions Log
| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-05-28 | v2 "terminal-luxe" system created | Evolve the dark/emerald DNA, replace Inter (convergence trap) with Satoshi/Geist/JetBrains Mono, lead with the "cannot drain" claim, and make the hero show the guardrail firing. Propagated via shared tokens so /demo, /docs, /proof inherit it. |
