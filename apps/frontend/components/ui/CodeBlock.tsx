"use client";

import { useState } from "react";
import { highlightShell } from "@/lib/highlight";

export function CodeBlock({ value }: { value: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <div className="bg-bg-elev border border-border rounded-md font-mono text-sm flex items-center justify-between px-4 py-3 group">
      <span className="truncate">
        <span className="text-syntax-punct">$ </span>
        {highlightShell(value)}
      </span>
      <button
        onClick={() => {
          navigator.clipboard.writeText(value);
          setCopied(true);
          setTimeout(() => setCopied(false), 1200);
        }}
        className="text-xs text-fg-muted opacity-0 group-hover:opacity-100 transition uppercase tracking-wider"
      >
        {copied ? "copied" : "copy"}
      </button>
    </div>
  );
}
