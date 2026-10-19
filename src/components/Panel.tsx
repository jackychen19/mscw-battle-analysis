import { forwardRef } from "react";
import type { SessionCommand, SessionState, View } from "../types";
import { ActionBar } from "./ActionBar";
import { Header } from "./Header";
import { StatsBox } from "./StatsBox";

const captions: Record<SessionState, string> = {
  idle: "LAST SESSION",
  recording: "LIVE",
  paused: "PAUSED",
  stopped: "SESSION ENDED",
};

export const Panel = forwardRef<HTMLDivElement, { view: View; onCommand: (c: SessionCommand) => void }>(
  function Panel({ view, onCommand }, ref) {
    const { snapshot } = view;
    const state = snapshot.state;
    // Idle shows the previous session's results, if any.
    const shown = state === "idle" ? view.lastSession : snapshot;
    const caption = shown === null ? "NO SESSIONS YET" : captions[state];
    return (
      <div className="panel" ref={ref}>
        <Header state={state} />
        <StatsBox snap={shown ?? snapshot} caption={caption} placeholder={shown === null} />
        <ActionBar state={state} onCommand={onCommand} />
        {view.mock && (
          <div className="mock-badge" title="Numbers come from a synthetic feed, not the game">
            MOCK DATA
          </div>
        )}
        {state !== "stopped" && <div className="footer">for MapleStory Classic World · unofficial</div>}
      </div>
    );
  },
);
