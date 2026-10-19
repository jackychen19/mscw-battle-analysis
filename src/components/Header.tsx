import type { SessionState } from "../types";
import { closeWindow } from "../api";
import { GearIcon, IconButton } from "./Buttons";

export function Header({ state }: { state: SessionState }) {
  return (
    <header className="header" data-tauri-drag-region>
      <h1 data-tauri-drag-region>BATTLE ANALYSIS</h1>
      {state === "recording" && <span className="status-pill pill-rec"><i className="rec-dot" />REC</span>}
      {state === "paused" && <span className="status-pill pill-paused">PAUSED</span>}
      {state === "stopped" && <span className="status-pill pill-results">RESULTS</span>}
      <div className="header-buttons">
        <IconButton tone="gray" label="Help">?</IconButton>
        <IconButton tone="green" label="Calibration">
          <GearIcon />
        </IconButton>
        <IconButton tone="orange" label="Close" onClick={() => void closeWindow()}>×</IconButton>
      </div>
    </header>
  );
}
