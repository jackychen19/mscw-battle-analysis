import type { Snapshot } from "../types";

const MIN = 60_000;

const base: Snapshot = {
  state: "idle",
  startLevel: null,
  level: null,
  exp: null,
  pctHundredths: null,
  durationMs: 0,
  pausedMs: 0,
  expGained: 0,
  expPerMin: 0,
  expPerMinRecent: 0,
  mesoGained: 0,
  mesoPerMin: 0,
  mesoPerMinRecent: 0,
  nextLevelMs: null,
  notificationExp: 0,
  expApproximate: false,
  mesoPossibleUndercount: false,
};

export const idleSnapshot: Snapshot = base;

export const recordingSnapshot: Snapshot = {
  ...base,
  state: "recording",
  startLevel: 59,
  level: 59,
  exp: 440_834,
  pctHundredths: 3843,
  durationMs: 12 * MIN + 34_000,
  expGained: 21_486,
  expPerMin: 1_710,
  expPerMinRecent: 1_824,
  mesoGained: 3_412,
  mesoPerMin: 271,
  mesoPerMinRecent: 288,
  nextLevelMs: 386 * MIN,
};

export const stoppedSnapshot: Snapshot = {
  ...base,
  state: "stopped",
  startLevel: 59,
  level: 60,
  exp: 18_220,
  pctHundredths: 148,
  durationMs: 61 * MIN + 12_000,
  pausedMs: 4 * MIN + 30_000,
  expGained: 724_406,
  expPerMin: 11_837,
  expPerMinRecent: 12_105,
  mesoGained: 18_934,
  mesoPerMin: 309,
  mesoPerMinRecent: 322,
  nextLevelMs: 101 * MIN,
};

export const mockStates: Record<string, Snapshot> = {
  idle: idleSnapshot,
  recording: recordingSnapshot,
  paused: { ...recordingSnapshot, state: "paused", pausedMs: 45_000 },
  stopped: stoppedSnapshot,
};
