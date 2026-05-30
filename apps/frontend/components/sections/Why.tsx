import {
  BadgeDollarSign,
  Fingerprint,
  KeyRound,
  Scale,
  ShieldCheck,
  Wallet,
} from "lucide-react";
import { Reveal } from "@/components/ui/Reveal";
import { TiltCard } from "@/components/ui/TiltCard";

const CARDS = [
  {
    title: "The agent never gets the credit line",
    body: "A tool call can ask to spend, but the local signer enforces the cap before any transaction is assembled.",
    icon: KeyRound,
  },
  {
    title: "The quote cannot mutate the asset",
    body: "The x402 requirement must match the devnet USDC mint and six decimals. Weird mints and decimal inflation are rejected.",
    icon: Fingerprint,
  },
  {
    title: "The browser demo is not the product path",
    body: "Production signing stays local. The hosted web stack exists to demonstrate the flow and publish proof, not custody funds.",
    icon: Wallet,
  },
  {
    title: "The audit log mirrors the wire",
    body: "Snake_case responses keep Rust, SDK output, CLI output, and ledger rows easy to compare during a review.",
    icon: Scale,
  },
] as const;

export function Why() {
  return (
    <section className="section-shell py-16 sm:py-20">
      <div className="grid gap-8 lg:grid-cols-[1fr_0.82fr] lg:items-start">
        <Reveal>
          <div className="section-kicker">
            <ShieldCheck className="h-3.5 w-3.5 text-accent" aria-hidden />
            Why it exists
          </div>
          <h2 className="max-w-3xl text-3xl font-bold leading-tight sm:text-4xl lg:text-[2.75rem]">
            Built for developers who are about to let agents touch money.
          </h2>
          <p className="mt-4 max-w-2xl text-sm leading-6 text-fg-muted sm:text-base">
            AgentsPay is not a broad wallet dashboard. It is a narrow security
            boundary around one job: let an AI agent pay for useful APIs while
            keeping the spend policy obvious and enforceable.
          </p>
        </Reveal>

        <Reveal delay={0.08} className="tool-panel p-5">
          <div className="flex items-center gap-3">
            <span className="grid h-10 w-10 place-items-center rounded-md border border-accent/20 bg-accent/10 text-accent">
              <BadgeDollarSign className="h-5 w-5" aria-hidden />
            </span>
            <div>
              <p className="font-mono text-xs uppercase text-fg-muted">
                Mainnet status
              </p>
              <p className="text-sm text-fg">Devnet today, v0.5 review first</p>
            </div>
          </div>
          <div className="mt-5 grid gap-2 font-mono text-xs">
            <StatusRow label="release" value="v0.3" />
            <StatusRow label="network" value="solana-devnet" />
            <StatusRow label="signing" value="local keypair" />
            <StatusRow label="mainnet" value="gated" />
          </div>
        </Reveal>
      </div>

      <div className="mt-8 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        {CARDS.map((card, index) => {
          const Icon = card.icon;
          return (
            <Reveal key={card.title} delay={index * 0.07} className="h-full">
              <TiltCard className="quiet-panel h-full min-h-[210px] p-5 transition-colors hover:border-accent/40 hover:bg-bg-panel/70">
                <Icon className="mb-5 h-5 w-5 text-accent" aria-hidden />
                <h3 className="text-sm font-semibold leading-6 text-fg">
                  {card.title}
                </h3>
                <p className="mt-3 text-xs leading-6 text-fg-muted">
                  {card.body}
                </p>
              </TiltCard>
            </Reveal>
          );
        })}
      </div>
    </section>
  );
}

function StatusRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-3 rounded-md border border-border bg-bg-deep px-3 py-2">
      <span className="text-fg-muted">{label}</span>
      <span className="text-syntax-string">{value}</span>
    </div>
  );
}
