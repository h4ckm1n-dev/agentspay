export function Footer() {
  return (
    <footer className="mx-auto mt-16 flex max-w-6xl flex-col justify-between gap-4 border-t border-border-subtle px-6 py-10 text-xs text-fg-dim sm:flex-row">
      <span>
        Open source · MIT · Built in <span className="text-fg">Rust</span> + <span className="text-fg">Next.js</span>
      </span>
      <span className="flex gap-4">
        <a href="https://github.com/h4ckm1n/agentspay" className="hover:text-fg transition">GitHub</a>
        <a href="/docs" className="hover:text-fg transition">Docs</a>
        <a href="/proof" className="hover:text-fg transition">Proof</a>
      </span>
    </footer>
  );
}
