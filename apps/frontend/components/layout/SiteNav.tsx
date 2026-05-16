"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { ExternalLink } from "lucide-react";
import { AgentsPayMark } from "@/components/brand/AgentsPayMark";

const LINKS = [
  { href: "/demo", label: "Demo" },
  { href: "/docs", label: "Docs" },
  { href: "/proof", label: "Proof" },
];

export function SiteNav() {
  const pathname = usePathname();

  return (
    <header className="sticky top-0 z-40 border-b border-border-subtle bg-bg/85 backdrop-blur">
      <nav className="mx-auto flex h-14 max-w-6xl items-center justify-between px-6">
        <Link href="/" className="flex items-center gap-2 font-mono text-sm">
          <AgentsPayMark className="h-7 w-7" />
          <span className="text-fg">agentspay</span>
        </Link>
        <div className="flex items-center gap-1">
          {LINKS.map((link) => {
            const active = pathname === link.href;
            return (
              <Link
                key={link.href}
                href={link.href}
                className={`rounded px-3 py-1.5 text-sm transition ${
                  active
                    ? "bg-bg-elev text-fg"
                    : "text-fg-muted hover:bg-bg-elev hover:text-fg"
                }`}
              >
                {link.label}
              </Link>
            );
          })}
          <a
            href="https://github.com/h4ckm1n/agentspay"
            className="ml-1 grid h-8 w-8 place-items-center rounded text-fg-muted transition hover:bg-bg-elev hover:text-fg"
            aria-label="GitHub"
          >
            <ExternalLink className="h-4 w-4" aria-hidden />
          </a>
        </div>
      </nav>
    </header>
  );
}
