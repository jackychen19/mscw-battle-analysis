import { forwardRef } from "react";
import type { SessionCommand, View } from "../types";
import { ActionBar } from "./ActionBar";
import { Header } from "./Header";
import { StatsBox } from "./StatsBox";

export const Panel = forwardRef<HTMLDivElement, { view: View; onCommand: (c: SessionCommand) => void }>(
  function Panel({ view, onCommand }, ref) {
    const { snapshot } = view;
    // Idle shows the previous session's results, if any.
    const shown = snapshot.state === "idle" ? view.lastSession : snapshot;
    return (
      <div className="panel" ref={ref}>
        <Header state={snapshot.state} />
        {snapshot.state === "idle" && <div className="caption">{view.lastSession ? "Last session" : "No sessions yet"}</div>}
        <StatsBox snap={shown ?? snapshot} placeholder={shown === null} />
        <ActionBar state={snapshot.state} onCommand={onCommand} />
      </div>
    );
  },
);
