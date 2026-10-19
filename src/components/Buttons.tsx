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
    <svg aria-hidden="true" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.6" strokeLinecap="round">
      <circle cx="12" cy="12" r="3.2" />
      <path d="M12 2.5v3M12 18.5v3M2.5 12h3M18.5 12h3M5.3 5.3l2.1 2.1M16.6 16.6l2.1 2.1M5.3 18.7l2.1-2.1M16.6 7.4l2.1-2.1" />
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
