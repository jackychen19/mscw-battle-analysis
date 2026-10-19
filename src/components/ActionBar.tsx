import type { SessionCommand, SessionState } from "../types";
import { PillButton, RoundButton, icons } from "./Buttons";

export function ActionBar({ state, canStart, onCommand }: { state: SessionState; canStart: boolean; onCommand: (c: SessionCommand) => void }) {
  switch (state) {
    case "idle":
      return (
        <PillButton tone="start" icon={icons.start} disabled={!canStart} onClick={() => onCommand("start")}>
          Start Analyzing
        </PillButton>
      );
    case "recording":
    case "paused":
      return (
        <div className="actions">
          {state === "recording" ? (
            <RoundButton label="Pause" onClick={() => onCommand("pause")}>{icons.pause}</RoundButton>
          ) : (
            <RoundButton label="Resume" disabled={!canStart} onClick={() => onCommand("resume")}>{icons.resume}</RoundButton>
          )}
          <PillButton tone="stop" icon={icons.stop} onClick={() => onCommand("stop")}>
            Stop
          </PillButton>
        </div>
      );
    case "stopped":
      return (
        <PillButton tone="start" icon={icons.restart} disabled={!canStart} onClick={() => onCommand("new")}>
          Start New Session
        </PillButton>
      );
  }
}
