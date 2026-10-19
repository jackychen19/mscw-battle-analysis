// Browser-only stand-in for the Tauri commands, so the UI can be developed
// with `npm run dev` and no Rust build. Numbers tick up while "recording".

import type { Backend } from "../api";
import type { Snapshot, View } from "../types";
import { idleSnapshot, mockStates, recordingSnapshot, stoppedSnapshot } from "./snapshots";

// `?mock=recording` etc. opens straight into a state; `?mock=empty` is idle with no history.
const param = new URLSearchParams(window.location.search).get("mock") ?? "idle";
let current: Snapshot = mockStates[param] ?? idleSnapshot;
let lastSession: Snapshot | null = param === "empty" ? null : stoppedSnapshot;
let lastTick = Date.now();

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
  return { snapshot: current, lastSession, mock: true };
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
