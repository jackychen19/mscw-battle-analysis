import { setMockSnapshot } from "./backend";
import { mockStates } from "./snapshots";

/** Browser dev mode only: jump straight to a mock state. */
export function DevStateSwitcher({ onChange }: { onChange: () => void }) {
  return (
    <div className="dev-switcher">
      <span>mock</span>
      {Object.entries(mockStates).map(([name, snap]) => (
        <button
          key={name}
          onClick={() => {
            setMockSnapshot(snap);
            onChange();
          }}
        >
          {name}
        </button>
      ))}
    </div>
  );
}
