import type { ButtonHTMLAttributes, ReactNode } from "react";

type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement>;

export function IconButton({ tone, label, children, ...rest }: { tone: "help" | "gear" | "close"; label: string; children: ReactNode } & ButtonProps) {
  return (
    <button className={`icon-btn icon-${tone}`} aria-label={label} title={label} {...rest}>
      {children}
    </button>
  );
}

export function PillButton({ tone, icon, children, ...rest }: { tone: "start" | "stop"; icon: ReactNode; children: ReactNode } & ButtonProps) {
  return (
    <button className={`pill-btn pill-${tone}`} {...rest}>
      {icon}
      {children}
    </button>
  );
}

export function RoundButton({ label, children, ...rest }: { label: string; children: ReactNode } & ButtonProps) {
  return (
    <button className="round-btn" aria-label={label} title={label} {...rest}>
      {children}
    </button>
  );
}

export const icons = {
  gear: (
    <svg aria-hidden="true" width="14" height="14" viewBox="0 0 24 24" fill="currentColor" fillRule="evenodd" stroke="currentColor" strokeWidth="1.2" strokeLinejoin="round">
      <path d="M19.11 9.32 L22.20 9.50 L22.20 14.50 L19.11 14.68 L18.92 15.14 L20.98 17.45 L17.45 20.98 L15.14 18.92 L14.68 19.11 L14.50 22.20 L9.50 22.20 L9.32 19.11 L8.86 18.92 L6.55 20.98 L3.02 17.45 L5.08 15.14 L4.89 14.68 L1.80 14.50 L1.80 9.50 L4.89 9.32 L5.08 8.86 L3.02 6.55 L6.55 3.02 L8.86 5.08 L9.32 4.89 L9.50 1.80 L14.50 1.80 L14.68 4.89 L15.14 5.08 L17.45 3.02 L20.98 6.55 L18.92 8.86Z M15.3 12a3.3 3.3 0 1 0 -6.6 0a3.3 3.3 0 1 0 6.6 0Z" />
    </svg>
  ),
  start: (
    <svg aria-hidden="true" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4">
      <circle cx="12" cy="12" r="9.5" />
      <circle cx="12" cy="12" r="4" fill="currentColor" />
    </svg>
  ),
  stop: (
    <svg aria-hidden="true" width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
      <rect x="5" y="5" width="14" height="14" rx="2" />
    </svg>
  ),
  restart: (
    <svg aria-hidden="true" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.6" strokeLinecap="round" strokeLinejoin="round">
      <path d="M20 12a8 8 0 1 1-2.4-5.7" />
      <path d="M20 4v5h-5" />
    </svg>
  ),
  pause: (
    <svg aria-hidden="true" width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
      <rect x="6" y="5" width="4" height="14" rx="1" />
      <rect x="14" y="5" width="4" height="14" rx="1" />
    </svg>
  ),
  resume: (
    <svg aria-hidden="true" width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
      <path d="M7 5v14l12-7z" />
    </svg>
  ),
};
