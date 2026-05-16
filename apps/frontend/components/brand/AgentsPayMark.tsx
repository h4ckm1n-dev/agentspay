export function AgentsPayMark({ className = "h-7 w-7" }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 64 64"
      className={className}
      role="img"
      aria-label="AgentsPay"
    >
      <rect
        x="4"
        y="4"
        width="56"
        height="56"
        rx="14"
        fill="#0a0a0b"
        stroke="#27272a"
        strokeWidth="2"
      />
      <path
        d="M20 43V25.5C20 20.3 24.3 16 29.5 16h5C39.7 16 44 20.3 44 25.5V43"
        fill="none"
        stroke="#10b981"
        strokeWidth="5"
        strokeLinecap="round"
      />
      <path
        d="M24 42h16"
        fill="none"
        stroke="#10b981"
        strokeWidth="5"
        strokeLinecap="round"
      />
      <path
        d="M24 31h16"
        fill="none"
        stroke="#7dd3fc"
        strokeWidth="4"
        strokeLinecap="round"
      />
      <circle cx="32" cy="31" r="6" fill="#0a0a0b" stroke="#7dd3fc" strokeWidth="3" />
      <circle cx="32" cy="31" r="2.5" fill="#34d399" />
      <path
        d="M18 48h28"
        fill="none"
        stroke="#34d399"
        strokeWidth="3"
        strokeLinecap="round"
        opacity="0.72"
      />
    </svg>
  );
}
