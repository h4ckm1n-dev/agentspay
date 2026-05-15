import type { Config } from "tailwindcss";

const config: Config = {
  content: ["./app/**/*.{ts,tsx}", "./components/**/*.{ts,tsx}"],
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
