export function Footer() {
  return (
    <footer className="border-t border-border-subtle mt-16 py-10 px-6 max-w-3xl mx-auto text-xs text-fg-dim flex flex-col sm:flex-row justify-between gap-4">
      <span>
        Open source · MIT · Built in <span className="text-fg">Rust</span> + <span className="text-fg">Next.js</span>
      </span>
      <span className="flex gap-4">
        <a href="https://github.com/user/agentspay" className="hover:text-fg transition">GitHub</a>
        <a href="https://x.com/user" className="hover:text-fg transition">X</a>
        <span>MCP registry (soon)</span>
      </span>
    </footer>
  );
}
