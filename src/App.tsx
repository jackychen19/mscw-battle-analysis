import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { backend, fitWindowHeight, inTauri } from "./api";
import { Panel } from "./components/Panel";
import { DevStateSwitcher } from "./mock/DevStateSwitcher";
import type { SessionCommand, View } from "./types";

const POLL_MS = 250;
const WINDOW_WIDTH = 316;
const WINDOW_MARGIN = 8; // room for the panel's drop shadow inside the transparent window

export default function App() {
  const [view, setView] = useState<View | null>(null);
  const [error, setError] = useState<string | null>(null);
  const panelRef = useRef<HTMLDivElement>(null);

  const refresh = useCallback(() => {
    backend.getView().then(setView, (e) => setError(String(e)));
  }, []);

  useEffect(() => {
    refresh();
    const id = setInterval(refresh, POLL_MS);
    return () => clearInterval(id);
  }, [refresh]);

  // Keep the native window exactly as tall as the panel.
  useLayoutEffect(() => {
    const el = panelRef.current;
    if (!el) return;
    const ro = new ResizeObserver(() => void fitWindowHeight(WINDOW_WIDTH, el.offsetHeight + WINDOW_MARGIN * 2));
    ro.observe(el);
    return () => ro.disconnect();
  }, [view !== null]);

  const onCommand = (command: SessionCommand) => {
    setError(null);
    backend.command(command).then(setView, (e) => setError(String(e)));
  };

  return (
    <main className="stage">
      {view && <Panel ref={panelRef} view={view} onCommand={onCommand} />}
      {error && <div className="error">{error}</div>}
      {!inTauri && <DevStateSwitcher onChange={refresh} />}
    </main>
  );
}
