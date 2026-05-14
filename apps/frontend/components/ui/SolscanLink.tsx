export function SolscanLink({ signature }: { signature: string }) {
  const short = `${signature.slice(0, 4)}…${signature.slice(-4)}`;
  return (
    <a
      href={`https://solscan.io/tx/${signature}?cluster=devnet`}
      target="_blank"
      rel="noreferrer"
      className="inline-flex items-center gap-2 font-mono text-xs text-accent hover:underline"
    >
      <span className="w-1.5 h-1.5 rounded-full bg-accent" />
      <span>solscan {short}</span>
    </a>
  );
}
