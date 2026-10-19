import type { SessionState } from "../types";
import { closeWindow, openCalibration } from "../api";
import { IconButton, icons } from "./Buttons";

export function Header({ state }: { state: SessionState }) {
  return (
    <header className="header" data-tauri-drag-region>
      <div className="header-title" data-tauri-drag-region>
        <h1 data-tauri-drag-region>BATTLE ANALYSIS</h1>
        {state === "recording" && <span className="status-pill pill-rec"><i className="rec-dot" />REC</span>}
        {state === "paused" && <span className="status-pill pill-paused">PAUSED</span>}
        {state === "stopped" && <span className="status-pill pill-results">RESULTS</span>}
      </div>
      <div className="header-buttons">
        <IconButton tone="help" label="Help">?</IconButton>
        {state === "idle" && (
          <IconButton tone="gear" label="Settings and calibration" onClick={() => void openCalibration()}>
            {icons.gear}
          </IconButton>
        )}
        <IconButton tone="close" label="Close" onClick={() => void closeWindow()}>×</IconButton>
      </div>
    </header>
  );
}
