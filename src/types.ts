// Mirrors battle_core::stats::Snapshot and the app's View (serde camelCase).

export type SessionState = "idle" | "recording" | "paused" | "stopped";

export interface Snapshot {
  state: SessionState;
  startLevel: number | null;
  level: number | null;
  exp: number | null;
  pctHundredths: number | null;
  durationMs: number;
  pausedMs: number;
  expGained: number;
  expPerMin: number;
  expPerMinRecent: number;
  mesoGained: number;
  mesoPerMin: number;
  mesoPerMinRecent: number;
  nextLevelMs: number | null;
  notificationExp: number;
  expApproximate: boolean;
  mesoPossibleUndercount: boolean;
}

export interface View {
  snapshot: Snapshot;
  lastSession: Snapshot | null;
}

export type SessionCommand = "start" | "pause" | "resume" | "stop" | "new";
