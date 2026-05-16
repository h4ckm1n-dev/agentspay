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
          DEFAULT: "#0a0a0b",
          elev: "#18181b",
          deep: "#000000",
        },
        border: {
          DEFAULT: "#27272a",
          subtle: "#1f1f23",
        },
        fg: {
          DEFAULT: "#fafafa",
          muted: "#a1a1aa",
          dim: "#71717a",
          faint: "#52525b",
        },
        accent: {
          DEFAULT: "#10b981",
          glow: "rgba(16,185,129,0.45)",
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
          // 6 domain-aware tokens (Solana / fintech)
          signature: "#c084fc", // cool-violet — Solana tx signature (base58 86-88)
          pubkey: "#7dd3fc", // sky-blue — Solana pubkey (base58 32-44)
          usdc: "#34d399", // mint-green — USDC amount (key-aware)
          timestamp: "#fcd34d", // amber — RFC3339 timestamp
          uuid: "#94a3b8", // dim cyan — UUID
          url: "#10b981", // accent-green — http(s):// URL
        },
      },
      fontFamily: {
        sans: [
          "Inter",
          "-apple-system",
          "BlinkMacSystemFont",
          "Segoe UI",
          "sans-serif",
        ],
        mono: [
          "ui-monospace",
          "SFMono-Regular",
          "Menlo",
          "Monaco",
          "Consolas",
          "monospace",
        ],
      },
      letterSpacing: {
        tight: "-0.02em",
      },
    },
  },
  plugins: [],
};

export default config;
