import type { SessionCommand, SessionState } from "../types";
import { PillButton, RoundButton } from "./Buttons";

export function ActionBar({ state, onCommand }: { state: SessionState; onCommand: (c: SessionCommand) => void }) {
  switch (state) {
    case "idle":
      return (
        <div className="actions">
          <PillButton tone="start" onClick={() => onCommand("start")}>Start Analyzing</PillButton>
        </div>
      );
    case "recording":
    case "paused":
      return (
        <div className="actions">
          {state === "recording" ? (
            <RoundButton label="Pause" onClick={() => onCommand("pause")}>
              <svg viewBox="0 0 12 12" width="12" height="12" aria-hidden="true">
                <rect x="2" y="1.5" width="2.8" height="9" rx="0.8" fill="currentColor" />
                <rect x="7.2" y="1.5" width="2.8" height="9" rx="0.8" fill="currentColor" />
              </svg>
            </RoundButton>
          ) : (
            <RoundButton label="Resume" onClick={() => onCommand("resume")}>
              <svg viewBox="0 0 12 12" width="12" height="12" aria-hidden="true">
                <path d="M3 1.5v9l7.5-4.5z" fill="currentColor" />
              </svg>
            </RoundButton>
          )}
          <PillButton tone="stop" onClick={() => onCommand("stop")}>Stop</PillButton>
        </div>
      );
    case "stopped":
      return (
        <div className="actions">
          <PillButton tone="start" onClick={() => onCommand("new")}>Start New Session</PillButton>
        </div>
      );
  }
}
