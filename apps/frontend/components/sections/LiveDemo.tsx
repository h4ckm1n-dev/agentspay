"use client";

import { useState } from "react";
import { FlaskConical, ServerCog, TerminalSquare } from "lucide-react";
import { SandboxTab } from "./SandboxTab";
import { DevnetTab } from "./DevnetTab";

type TabKey = "sandbox" | "devnet";

export function LiveDemo() {
  const [active, setActive] = useState<TabKey>("sandbox");

  return (
    <section id="demo" className="section-shell py-12 sm:py-16">
      <div className="mb-7 grid gap-5 lg:grid-cols-[1fr_auto] lg:items-end">
        <div>
          <div className="section-kicker">
            <FlaskConical className="h-3.5 w-3.5 text-accent" aria-hidden />
            Live demo
          </div>
          <h2 className="max-w-3xl text-3xl font-semibold leading-tight sm:text-4xl">
            Exercise the wallet boundary from the browser.
          </h2>
          <p className="mt-3 max-w-2xl text-sm leading-6 text-fg-muted">
            Sandbox mode is isolated and fast. Devnet mode asks the provider for
            an x402 quote first, then signs a real USDC transfer.
          </p>
        </div>

        <div className="inline-flex w-full rounded-lg border border-border bg-black/30 p-1 sm:w-auto">
          <TabButton
            active={active === "sandbox"}
            onClick={() => setActive("sandbox")}
            icon={<TerminalSquare className="h-4 w-4" aria-hidden />}
          >
            Sandbox
          </TabButton>
          <TabButton
            active={active === "devnet"}
            onClick={() => setActive("devnet")}
            icon={<ServerCog className="h-4 w-4" aria-hidden />}
          >
            Devnet
          </TabButton>
        </div>
      </div>

      {active === "sandbox" ? <SandboxTab /> : <DevnetTab />}
    </section>
  );
}

function TabButton({
  active,
  onClick,
  icon,
  children,
}: {
  active: boolean;
  onClick: () => void;
  icon: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`flex min-h-10 flex-1 items-center justify-center gap-2 rounded-md px-4 py-2 text-sm transition sm:flex-none ${
        active
          ? "bg-fg text-bg"
          : "text-fg-muted hover:bg-bg-elev hover:text-fg"
      }`}
    >
      {icon}
      {children}
    </button>
  );
}
