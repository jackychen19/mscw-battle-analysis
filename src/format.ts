import type { View } from "./types";

export function formatDuration(ms: number): string {
  const total = Math.floor(ms / 1000);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  return [h, m, s].map((n) => String(n).padStart(2, "0")).join(":");
}

/** `6m 40s`, `1h 2m`, `0s`. */
export function formatShortDuration(ms: number): string {
  const total = Math.floor(ms / 1000);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  if (h > 0) return `${h}h ${m}m`;
  if (m > 0) return `${m}m ${s}s`;
  return `${s}s`;
}

/** Estimated time, e.g. `~1h 41m`. */
export function formatEta(ms: number | null): string {
  if (ms === null) return "—";
  const totalMin = Math.round(ms / 60_000);
  if (totalMin < 1) return "< 1m";
  const d = Math.floor(totalMin / 1440);
  const h = Math.floor((totalMin % 1440) / 60);
  const m = totalMin % 60;
  if (d > 0) return `~${d}d ${h}h`;
  if (h > 0) return `~${h}h ${m}m`;
  return `~${m}m`;
}

const int = new Intl.NumberFormat("en-US", { maximumFractionDigits: 0 });

export function formatInt(n: number): string {
  return int.format(n);
}

export function formatPct(hundredths: number | null): string {
  return hundredths === null ? "—" : `${(hundredths / 100).toFixed(2)}%`;
}

/** Why Start is disabled. */
export function captureNotice(view: View): string {
  const { capture } = view;
  if (capture.permission === "denied") return "Screen Recording not allowed · open ⚙ to fix";
  if (capture.game === null) return view.autoPaused ? "MapleStory window lost · paused" : "MapleStory not detected";
  return "Not calibrated · open ⚙ to set up";
}
