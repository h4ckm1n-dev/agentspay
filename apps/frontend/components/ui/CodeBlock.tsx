"use client";

import { useState } from "react";
import { Check, Copy } from "lucide-react";
import { highlightShell } from "@/lib/highlight";

export function CodeBlock({ value }: { value: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <div className="group relative overflow-hidden rounded-md border border-border bg-bg-deep font-mono text-sm shadow-[inset_0_1px_0_rgba(255,255,255,0.03)]">
      <pre className="max-h-[260px] overflow-auto whitespace-pre-wrap px-4 py-3 pr-12 text-xs leading-6 sm:text-sm">
        <span className="text-syntax-punct">$ </span>
        {highlightShell(value)}
      </pre>
      <button
        type="button"
        aria-label={copied ? "Copied" : "Copy command"}
        onClick={() => {
          navigator.clipboard.writeText(value);
          setCopied(true);
          setTimeout(() => setCopied(false), 1200);
        }}
        className="absolute right-2 top-2 grid h-8 w-8 place-items-center rounded-md border border-border bg-bg-panel text-fg-muted opacity-100 transition hover:text-fg sm:opacity-0 sm:group-hover:opacity-100"
      >
        {copied ? (
          <Check className="h-4 w-4 text-accent" aria-hidden />
        ) : (
          <Copy className="h-4 w-4" aria-hidden />
        )}
      </button>
    </div>
  );
}
