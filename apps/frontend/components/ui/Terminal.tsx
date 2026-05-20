"use client";

import { highlightJson } from "@/lib/highlight";

export interface TerminalLine {
  kind: "cmd" | "out" | "err" | "ok";
  text: string;
}

export function Terminal({ lines }: { lines: TerminalLine[] }) {
  return (
    <div className="overflow-hidden rounded-lg border border-border bg-black/40 font-mono text-xs leading-relaxed shadow-[0_24px_90px_rgba(0,0,0,0.28)]">
      <div className="flex items-center justify-between border-b border-border-subtle bg-bg-elev/80 px-3 py-2">
        <div className="flex items-center gap-2">
          <span className="h-2.5 w-2.5 rounded-full bg-red-400" />
          <span className="h-2.5 w-2.5 rounded-full bg-yellow-400" />
          <span className="h-2.5 w-2.5 rounded-full bg-green-400" />
          <span className="ml-3 text-xs text-fg-muted">~/agentspay $</span>
        </div>
        <span className="text-xs text-fg-faint">mcp/v0.3</span>
      </div>
      <div className="min-h-[260px] max-h-[460px] overflow-y-auto bg-bg-deep p-4">
        {lines.length === 0 ? (
          <div className="grid min-h-[220px] place-items-center text-center">
            <p className="max-w-xs text-fg-faint">
              Select a tool or payment stage. The raw MCP and x402 output will
              stream here.
            </p>
          </div>
        ) : (
          lines.map((l, i) => <TerminalRow key={i} line={l} />)
        )}
      </div>
    </div>
  );
}

function TerminalRow({ line }: { line: TerminalLine }) {
  if (line.kind === "cmd") {
    return (
      <div className="text-fg-muted whitespace-pre-wrap">
        <span className="text-syntax-punct">$ </span>
        {line.text}
      </div>
    );
  }
  if (line.kind === "ok") {
    return <div className="text-accent whitespace-pre-wrap">{line.text}</div>;
  }
  if (line.kind === "err") {
    return <div className="text-red-400 whitespace-pre-wrap">{line.text}</div>;
  }
  // out - try to syntax-highlight JSON-shaped payloads
  const trimmed = line.text.trimStart();
  if (trimmed.startsWith("{") || trimmed.startsWith("[")) {
    return (
      <pre className="text-fg whitespace-pre-wrap font-mono m-0">
        {highlightJson(line.text)}
      </pre>
    );
  }
  return <div className="text-fg whitespace-pre-wrap">{line.text}</div>;
}
