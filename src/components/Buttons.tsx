import type { ButtonHTMLAttributes, ReactNode } from "react";

type Tone = "green" | "orange" | "gray";

export function IconButton({ tone, label, children, ...rest }: { tone: Tone; label: string; children: ReactNode } & ButtonHTMLAttributes<HTMLButtonElement>) {
  return (
    <button className={`icon-btn tone-${tone}`} aria-label={label} title={label} {...rest}>
      {children}
    </button>
  );
}

export function PillButton({ tone, children, ...rest }: { tone: "start" | "stop"; children: ReactNode } & ButtonHTMLAttributes<HTMLButtonElement>) {
  return (
    <button className={`pill-btn pill-${tone}`} {...rest}>
      {children}
    </button>
  );
}

export function RoundButton({ label, children, ...rest }: { label: string; children: ReactNode } & ButtonHTMLAttributes<HTMLButtonElement>) {
  return (
    <button className="round-btn" aria-label={label} title={label} {...rest}>
      {children}
    </button>
  );
}

export function GearIcon() {
  return (
    <svg viewBox="0 0 16 16" width="11" height="11" aria-hidden="true">
      <path
        fill="currentColor"
        d="M6.9 1h2.2l.3 1.8 1.2.5 1.5-1 1.6 1.6-1 1.5.5 1.2L15 6.9v2.2l-1.8.3-.5 1.2 1 1.5-1.6 1.6-1.5-1-1.2.5-.3 1.8H6.9l-.3-1.8-1.2-.5-1.5 1-1.6-1.6 1-1.5-.5-1.2L1 9.1V6.9l1.8-.3.5-1.2-1-1.5 1.6-1.6 1.5 1 1.2-.5zM8 5.6a2.4 2.4 0 1 0 0 4.8 2.4 2.4 0 0 0 0-4.8"
      />
    </svg>
  );
}
