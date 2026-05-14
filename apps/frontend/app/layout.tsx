import type { Metadata } from "next";

import "./globals.css";

export const metadata: Metadata = {
  title: "AgentsPay Dashboard",
  description: "Spend controls and x402-compatible payments for autonomous agents."
};

interface RootLayoutProps {
  children: React.ReactNode;
}

export default function RootLayout({ children }: RootLayoutProps) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
