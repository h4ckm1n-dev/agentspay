export function SolscanLink({ signature }: { signature: string }) {
  const short = `${signature.slice(0, 4)}...${signature.slice(-4)}`;
  return (
    <a
      href={`https://solscan.io/tx/${signature}?cluster=devnet`}
      target="_blank"
      rel="noreferrer"
      className="inline-flex min-h-10 w-full items-center justify-center gap-2 rounded-md border border-accent/30 bg-accent/10 px-3 font-mono text-xs text-accent transition hover:border-accent/60 hover:bg-accent/20"
    >
      <span className="h-1.5 w-1.5 rounded-full bg-accent" />
      <span>solscan {short}</span>
    </a>
  );
}
