"use client";

import { useState } from "react";
import { SandboxTab } from "./SandboxTab";

type TabKey = "sandbox" | "devnet";

export function LiveDemo() {
  const [active, setActive] = useState<TabKey>("sandbox");

  return (
    <section
      id="demo"
      className="px-6 py-16 max-w-3xl mx-auto border-t border-border-subtle"
    >
      <p className="text-xs uppercase tracking-[0.12em] text-accent mb-4 font-mono">
        §3 · LIVE DEMO
      </p>
      <h2 className="text-2xl sm:text-3xl font-semibold tracking-tight mb-6">
        Run it in your browser.
      </h2>

      <div className="flex gap-0 border-b border-border mb-6">
        <TabButton
          active={active === "sandbox"}
          onClick={() => setActive("sandbox")}
        >
          Sandbox
        </TabButton>
        <TabButton
          active={active === "devnet"}
          onClick={() => setActive("devnet")}
        >
          Devnet (real on-chain)
        </TabButton>
      </div>

      {active === "sandbox" ? (
        <SandboxTab />
      ) : (
        <p className="text-fg-muted text-sm">Devnet tab arrives in Phase 6.</p>
      )}
    </section>
  );
}

function TabButton({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      className={`px-4 py-2 text-sm border-b-2 -mb-px transition ${
        active
          ? "text-fg border-accent"
          : "text-fg-dim border-transparent hover:text-fg-muted"
      }`}
    >
      {children}
    </button>
  );
}
