const FALLBACK_SITE_URL = "https://agentspay.dev";

function resolveSiteUrl(): string {
  const raw =
    process.env.NEXT_PUBLIC_SITE_URL ??
    (process.env.VERCEL_URL
      ? `https://${process.env.VERCEL_URL}`
      : undefined) ??
    FALLBACK_SITE_URL;
  return raw.replace(/\/$/, "");
}

export const SITE = {
  url: resolveSiteUrl(),
  name: "AgentsPay",
  shortName: "agentspay",
  tagline: "Budget-controlled USDC for AI agents",
  description:
    "AgentsPay is a local MCP server that gives Claude Code, Cursor, Cline, and Zed agents a budget-controlled USDC wallet for x402-priced APIs. Per-call and daily caps are enforced before signing; every settlement is recorded in SQLite and verifiable on Solana.",
  shortDescription:
    "An MCP server that gives your AI agent a budget-controlled USDC wallet for x402 APIs.",
  twitter: "@agentspay",
  github: "https://github.com/h4ckm1n/agentspay",
  repoOwnerName: "h4ckm1n",
  themeColor: "#0a0a0b",
  locale: "en_US",
  ogImagePath: "/opengraph-image",
  keywords: [
    "MCP",
    "Model Context Protocol",
    "x402",
    "AI agent wallet",
    "USDC",
    "Solana",
    "agentic payments",
    "autonomous agents",
    "Claude Code",
    "Cursor",
    "Cline",
    "budget-controlled wallet",
  ],
} as const;

export function absoluteUrl(path: string): string {
  if (path.startsWith("http")) return path;
  return `${SITE.url}${path.startsWith("/") ? path : `/${path}`}`;
}
