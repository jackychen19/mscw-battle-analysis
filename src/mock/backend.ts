// Browser-only stand-in for the Tauri commands, so the UI can be developed
// with `npm run dev` and no Rust build. Numbers tick up while "recording".

import type { Backend, Setup } from "../api";
import type { CaptureStatus, Image, Missing, RectPt, Regions, Snapshot, View, WindowInfo } from "../types";
import { idleSnapshot, mockStates, recordingSnapshot, stoppedSnapshot } from "./snapshots";

// `?mock=recording` etc. opens straight into a state; `?mock=empty` is idle with no history.
const param = new URLSearchParams(window.location.search).get("mock") ?? "idle";
let current: Snapshot = mockStates[param] ?? idleSnapshot;
let lastSession: Snapshot | null = param === "empty" ? null : stoppedSnapshot;
let lastTick = Date.now();
// `?game=none` shows the "MapleStory not detected" state.
const noGame = new URLSearchParams(window.location.search).get("game") === "none";
// `?game=learning` shows automatic calibration in progress.
const learningParam = new URLSearchParams(window.location.search).get("game") === "learning";
const capture: CaptureStatus = {
  permission: "granted",
  game: noGame ? null : "MapleStory",
  calibrated: !noGame && !learningParam,
  learning: learningParam ? { found: true, samples: 2, needed: 4 } : null,
};

function tick(): void {
  const now = Date.now();
  const dt = now - lastTick;
  lastTick = now;
  if (current.state === "recording") {
    current = { ...current, durationMs: current.durationMs + dt, expGained: current.expGained + Math.round((dt / 60_000) * current.expPerMin) };
  } else if (current.state === "paused") {
    current = { ...current, pausedMs: current.pausedMs + dt };
  }
}

function view(): View {
  tick();
  return { snapshot: current, lastSession, mock: true, capture, canStart: capture.calibrated, autoPaused: false };
}

export const mockBackend: Backend = {
  async getView() {
    return view();
  },
  async command(command) {
    tick();
    switch (command) {
      case "start":
        current = { ...recordingSnapshot };
        break;
      case "pause":
        current = { ...current, state: "paused" };
        break;
      case "resume":
        current = { ...current, state: "recording" };
        break;
      case "stop":
        current = { ...current, state: "stopped" };
        break;
      case "new":
        lastSession = current;
        current = { ...recordingSnapshot, durationMs: 0, expGained: 0, mesoGained: 0 };
        break;
    }
    return view();
  },
};

/** Force a state for the dev switcher. */
export function setMockSnapshot(s: Snapshot): void {
  current = s;
}

// ---- Calibration window ----

const mockWindow: WindowInfo = {
  id: 1,
  title: "MapleStory",
  owner: "MapleStory",
  bounds: { x: 0, y: 0, w: 800, h: 600 },
  scale: 2,
  onScreen: true,
};
let mockRegions: Regions | null = null;
let mockMissing: Missing | null = null;

/** A dark window with a lighter strip where the EXP bar would be. */
function fakeImage(w: number, h: number): Image {
  const rgba = new Uint8ClampedArray(w * h * 4);
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const i = (y * w + x) * 4;
      const bar = y > h - 30 && y < h - 18;
      const v = bar ? 70 : 30 + Math.round((40 * y) / h);
      rgba.set([v, v + 8, v + 20, 255], i);
    }
  }
  return { width: w, height: h, rgba };
}

export const mockSetup: Setup = {
  async status() {
    return { permission: "granted", windows: [mockWindow], game: mockWindow, regions: mockRegions, missing: mockMissing };
  },
  async requestPermission() {
    return "granted";
  },
  async openPermissionSettings() {},
  async chooseWindow() {
    return mockWindow;
  },
  async windowPreview() {
    return fakeImage(mockWindow.bounds.w, mockWindow.bounds.h);
  },
  async regionPreview(r: RectPt) {
    return fakeImage(Math.max(1, Math.round(r.w * 2)), Math.max(1, Math.round(r.h * 2)));
  },
  async teach(regions, sample) {
    mockRegions = regions;
    const typed = `${sample.exp},.[]%${sample.pctHundredths}`;
    const prev = mockMissing ?? { exp: "0123456789,.[]%", level: "0123456789" };
    mockMissing = {
      exp: [...prev.exp].filter((c) => !typed.includes(c)).join(""),
      level: [...prev.level].filter((c) => !String(sample.level).includes(c)).join(""),
    };
    return mockMissing;
  },
  async reset() {
    mockRegions = null;
    mockMissing = null;
  },
};
