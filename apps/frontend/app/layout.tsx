import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "AgentsPay — A budget-controlled USDC wallet for AI agents",
  description:
    "One MCP install. Real Solana settlement. Per-call and daily caps enforced before the chain.",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
