import { ImageResponse } from "next/og";
import { SITE } from "@/lib/seo";

export const runtime = "edge";
export const alt = `${SITE.name} — ${SITE.tagline}`;
export const size = { width: 1200, height: 630 };
export const contentType = "image/png";

export default function OpenGraphImage() {
  return new ImageResponse(
    <div
      style={{
        width: "100%",
        height: "100%",
        display: "flex",
        flexDirection: "column",
        justifyContent: "space-between",
        padding: "72px",
        background:
          "linear-gradient(135deg, #0a0a0b 0%, #111114 60%, #1a1a20 100%)",
        color: "#fafafa",
        fontFamily: "monospace",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: "16px" }}>
        <div
          style={{
            width: 56,
            height: 56,
            borderRadius: 12,
            background: "#34d399",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            fontSize: 32,
            fontWeight: 800,
            color: "#0a0a0b",
          }}
        >
          A
        </div>
        <span
          style={{
            fontSize: 28,
            color: "#a3a3a3",
            letterSpacing: "0.04em",
          }}
        >
          agentspay
        </span>
      </div>

      <div style={{ display: "flex", flexDirection: "column", gap: "20px" }}>
        <div
          style={{
            fontSize: 22,
            color: "#34d399",
            letterSpacing: "0.18em",
            textTransform: "uppercase",
          }}
        >
          v0.3 · solana devnet
        </div>
        <div
          style={{
            fontSize: 80,
            fontWeight: 700,
            lineHeight: 1.05,
            letterSpacing: "-0.02em",
            color: "#fafafa",
            maxWidth: "1000px",
          }}
        >
          Budget-controlled USDC for AI agents.
        </div>
        <div
          style={{
            fontSize: 28,
            color: "#a3a3a3",
            lineHeight: 1.35,
            maxWidth: "900px",
          }}
        >
          One MCP install. Real Solana settlement. Per-call and daily caps
          enforced before the chain.
        </div>
      </div>

      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "flex-end",
          fontSize: 22,
          color: "#737378",
        }}
      >
        <span>github.com/h4ckm1n/agentspay</span>
        <span style={{ color: "#34d399" }}>agentspay.dev</span>
      </div>
    </div>,
    { ...size },
  );
}
