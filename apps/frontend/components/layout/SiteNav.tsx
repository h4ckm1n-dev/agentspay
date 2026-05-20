"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { BookOpen, ExternalLink, FlaskConical, ReceiptText } from "lucide-react";
import { AgentsPayMark } from "@/components/brand/AgentsPayMark";

const LINKS = [
  { href: "/demo", label: "Demo", icon: FlaskConical },
  { href: "/docs", label: "Docs", icon: BookOpen },
  { href: "/proof", label: "Proof", icon: ReceiptText },
];

export function SiteNav() {
  const pathname = usePathname();

  return (
    <header className="sticky top-0 z-40 border-b border-white/5 bg-bg/80 backdrop-blur-xl">
      <nav className="mx-auto flex h-16 max-w-7xl items-center justify-between px-4 sm:px-6 lg:px-8">
        <Link
          href="/"
          className="flex min-w-0 items-center gap-2 rounded-md pr-2 font-mono text-sm"
          aria-label="AgentsPay home"
        >
          <AgentsPayMark className="h-8 w-8 shrink-0" />
          <span className="hidden text-fg sm:inline">agentspay</span>
          <span className="hidden rounded border border-border bg-bg-panel px-1.5 py-0.5 text-[10px] uppercase text-fg-muted md:inline">
            v0.3
          </span>
        </Link>
        <div className="flex items-center gap-1 rounded-md border border-white/5 bg-black/30 p-1">
          {LINKS.map((link) => {
            const active = pathname === link.href;
            const Icon = link.icon;
            return (
              <Link
                key={link.href}
                href={link.href}
                className={`rounded px-3 py-1.5 text-sm transition ${
                  active
                    ? "bg-fg text-bg"
                    : "text-fg-muted hover:bg-bg-elev hover:text-fg"
                }`}
              >
                <span className="flex items-center gap-2">
                  <Icon className="hidden h-3.5 w-3.5 sm:block" aria-hidden />
                  {link.label}
                </span>
              </Link>
            );
          })}
          <a
            href="https://github.com/h4ckm1n-dev/agentspay"
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
