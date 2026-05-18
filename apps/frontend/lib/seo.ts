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
  tagline: "A USDC wallet your AI agent cannot drain",
  description:
    "AgentsPay is an open-source MCP server that gives Claude Code, Cursor, Cline, and Zed agents a USDC wallet they cannot drain. Per-call and daily caps are checked before signing. Settles on Solana in ~2 seconds. Security-audited (4 critical bugs caught and fixed) with 46 Rust + 10 TypeScript tests in CI. Also ships as @agentspay/sdk-js for Node and @agentspay/cli for shell.",
  shortDescription:
    "Open source MCP wallet your AI agent cannot drain. Per-call + daily caps, Solana settlement, security-audited.",
  twitter: "@agentspay",
  github: "https://github.com/h4ckm1n-dev/agentspay",
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
    "Zed",
    "budget-controlled wallet",
    "self-custodial",
    "open source",
    "security audited",
    "agentspay",
  ],
} as const;

export function absoluteUrl(path: string): string {
  if (path.startsWith("http")) return path;
  return `${SITE.url}${path.startsWith("/") ? path : `/${path}`}`;
}
