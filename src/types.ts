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
  expApproximate: boolean;
  mesoPossibleUndercount: boolean;
}

export type Permission = "granted" | "denied";

export interface CaptureStatus {
  permission: Permission;
  /** Title of the game window, when it's open. */
  game: string | null;
  /** Calibrated for this window at its current scale. */
  calibrated: boolean;
}

export interface View {
  snapshot: Snapshot;
  lastSession: Snapshot | null;
  /** True while the numbers come from a synthetic feed rather than the game. */
  mock: boolean;
  capture: CaptureStatus;
  /** Game found and calibrated (always true with the mock feed). */
  canStart: boolean;
  /** Paused automatically because the game window went away. */
  autoPaused: boolean;
}

// Mirrors battle_core::capture / calibration and the app's calibration commands.

/** Points, relative to the top-left of the game window. */
export interface RectPt {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface WindowInfo {
  id: number;
  title: string;
  owner: string;
  bounds: RectPt;
  scale: number;
}

export interface Regions {
  expText: RectPt;
  level: RectPt;
}

/** Characters not taught yet. */
export interface Missing {
  exp: string;
  level: string;
}

export interface SetupStatus {
  permission: Permission;
  windows: WindowInfo[];
  game: WindowInfo | null;
  regions: Regions | null;
  missing: Missing | null;
}

export interface Sample {
  level: number;
  exp: number;
  pctHundredths: number;
}

/** An RGBA image decoded from the backend's `[w][h][rgba…]` encoding. */
export interface Image {
  width: number;
  height: number;
  rgba: Uint8ClampedArray;
}

export type SessionCommand = "start" | "pause" | "resume" | "stop" | "new";
