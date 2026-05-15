"use client";

import { highlightJson } from "@/lib/highlight";

export interface TerminalLine {
  kind: "cmd" | "out" | "err" | "ok";
  text: string;
}

export function Terminal({ lines }: { lines: TerminalLine[] }) {
  return (
    <div className="border border-border rounded-md overflow-hidden font-mono text-xs leading-relaxed">
      {/* title bar */}
      <div className="bg-bg-elev border-b border-border-subtle flex items-center justify-between px-3 py-2">
        <div className="flex items-center gap-2">
          <span className="w-2.5 h-2.5 rounded-full bg-red-400" />
          <span className="w-2.5 h-2.5 rounded-full bg-yellow-400" />
          <span className="w-2.5 h-2.5 rounded-full bg-green-400" />
          <span className="ml-3 text-fg-muted text-xs">~/agentspay $</span>
        </div>
        <span className="text-fg-faint text-xs">mcp/v0.3</span>
      </div>
      {/* body */}
      <div className="bg-bg-deep p-4 min-h-[180px] max-h-[360px] overflow-y-auto">
        {lines.length === 0 ? (
          <p className="text-fg-faint">
            Click a tool above to see its output here.
          </p>
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
  // out — try to syntax-highlight JSON-shaped payloads
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
