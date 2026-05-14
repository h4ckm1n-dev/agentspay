"use client";

export interface TerminalLine {
  kind: "cmd" | "out" | "err" | "ok";
  text: string;
}

export function Terminal({ lines }: { lines: TerminalLine[] }) {
  return (
    <div className="bg-bg-deep border border-border rounded-md font-mono text-xs leading-relaxed p-4 min-h-[180px] max-h-[360px] overflow-y-auto">
      {lines.length === 0 ? (
        <p className="text-fg-faint">
          Click a tool above to see its output here.
        </p>
      ) : (
        lines.map((l, i) => (
          <div
            key={i}
            className={
              l.kind === "cmd"
                ? "text-fg-muted"
                : l.kind === "out"
                  ? "text-fg"
                  : l.kind === "ok"
                    ? "text-accent"
                    : "text-red-400"
            }
          >
            {l.kind === "cmd" ? `$ ${l.text}` : l.text}
          </div>
        ))
      )}
    </div>
  );
}
