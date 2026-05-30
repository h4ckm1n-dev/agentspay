import type { Config } from "tailwindcss";

const config: Config = {
  content: [
    "./app/**/*.{ts,tsx}",
    "./components/**/*.{ts,tsx}",
    "./lib/**/*.{ts,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        bg: {
          DEFAULT: "#070708",
          elev: "#0d0d10",
          deep: "#000000",
          panel: "#101014",
          raised: "#17171c",
        },
        border: {
          DEFAULT: "#27272a",
          subtle: "#1c1c20",
          strong: "#33333a",
        },
        fg: {
          DEFAULT: "#fafafa",
          muted: "#a1a1aa",
          dim: "#71717a",
          faint: "#5b5b63",
        },
        accent: {
          DEFAULT: "#10b981",
          mint: "#34d399",
          sky: "#38bdf8",
          gold: "#fbbf24",
          glow: "rgba(16,185,129,0.45)",
        },
        // Reserved for the single most important state: a guardrail blocking a
        // payment. Never used decoratively.
        gate: {
          DEFAULT: "#fbbf24",
          soft: "rgba(251,191,36,0.08)",
          line: "rgba(251,191,36,0.35)",
        },
        terminal: {
          green: "#a1f87f",
        },
        syntax: {
          key: "#7dd3fc", // cyan — JSON keys, shell flags
          string: "#a1f87f", // green — string values
          number: "#fbbf24", // orange — numbers
          bool: "#c084fc", // purple — true / false
          null: "#f87171", // dim red — null
          punct: "#71717a", // dim grey — { } [ ] , : $ =
          cmd: "#fafafa", // bold white — shell command word
          signature: "#c084fc", // cool-violet — Solana tx signature (base58 86-88)
          pubkey: "#7dd3fc", // sky-blue — Solana pubkey (base58 32-44)
          usdc: "#34d399", // mint-green — USDC amount (key-aware)
          timestamp: "#fcd34d", // amber — RFC3339 timestamp
          uuid: "#94a3b8", // dim cyan — UUID
          url: "#10b981", // accent-green — http(s):// URL
        },
      },
      fontFamily: {
        display: ["var(--font-display)", "Satoshi", "system-ui", "sans-serif"],
        sans: ["var(--font-sans)", "system-ui", "sans-serif"],
        mono: [
          "var(--font-mono)",
          "ui-monospace",
          "SFMono-Regular",
          "monospace",
        ],
      },
      letterSpacing: {
        tightest: "-0.04em",
      },
      borderRadius: {
        xl2: "1.125rem",
      },
      boxShadow: {
        lift: "0 40px 120px rgba(0,0,0,0.55)",
        panel: "0 24px 90px rgba(0,0,0,0.34)",
      },
      keyframes: {
        "fade-up": {
          "0%": { opacity: "0", transform: "translateY(14px)" },
          "100%": { opacity: "1", transform: "translateY(0)" },
        },
        "aurora-drift": {
          "0%, 100%": { transform: "translate3d(-2%, 0, 0) scale(1)" },
          "50%": { transform: "translate3d(2%, -3%, 0) scale(1.06)" },
        },
        marquee: {
          "0%": { transform: "translateX(0)" },
          "100%": { transform: "translateX(-50%)" },
        },
        "gradient-shift": {
          "0%, 100%": { backgroundPosition: "0% 50%" },
          "50%": { backgroundPosition: "100% 50%" },
        },
      },
      animation: {
        "fade-up": "fade-up 0.6s cubic-bezier(0.22,1,0.36,1) both",
        aurora: "aurora-drift 18s ease-in-out infinite",
        marquee: "marquee 38s linear infinite",
        gradient: "gradient-shift 6s ease-in-out infinite",
      },
    },
  },
  plugins: [],
};

export default config;
