import type { Snapshot } from "../types";
import { formatDuration, formatEta, formatInt, formatPct } from "../format";

/** `placeholder` renders dashes for values (idle with no previous session). */
export function StatsBox({ snap, placeholder = false }: { snap: Snapshot; placeholder?: boolean }) {
  const dash = (v: string) => (placeholder ? "—" : v);
  const stopped = snap.state === "stopped";
  const pct = snap.pctHundredths ?? 0;
  const levelUp = snap.startLevel !== null && snap.level !== null && snap.level > snap.startLevel;

  return (
    <section className="stats-box">
      <div className="level-line">
        <span className="level">
          {snap.level === null || placeholder ? (
            "Lv. —"
          ) : stopped && levelUp ? (
            <>Lv. {snap.startLevel} <span className="arrow">→</span> {snap.level}</>
          ) : (
            <>Lv. {snap.level}</>
          )}
        </span>
        <span className="pct">{dash(formatPct(snap.pctHundredths))}</span>
      </div>
      <div className="progress" role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={pct / 100}>
        <div className="progress-fill" style={{ width: `${placeholder ? 0 : pct / 100}%` }} />
      </div>

      <dl className="rows">
        <Row label="Battle Duration" value={dash(formatDuration(snap.durationMs))} />
        {stopped && snap.pausedMs > 0 && (
          <Sub label="paused (excluded)" value={formatDuration(snap.pausedMs)} />
        )}
        <Row label="EXP Acquired" value={dash(formatInt(snap.expGained))} flag={snap.expApproximate ? "≈" : undefined} />
        <Sub label="per min" value={dash(formatInt(snap.expPerMin))} recent={placeholder ? undefined : snap.expPerMinRecent} />
        <Row label="Mesos Acquired" value={dash(formatInt(snap.mesoGained))} flag={snap.mesoPossibleUndercount ? "≥" : undefined} />
        <Sub label="per min" value={dash(formatInt(snap.mesoPerMin))} recent={placeholder ? undefined : snap.mesoPerMinRecent} />
        <Row label="Next Level" value={dash(formatEta(snap.nextLevelMs))} />
      </dl>
    </section>
  );
}

function Row({ label, value, flag }: { label: string; value: string; flag?: string }) {
  return (
    <div className="row">
      <dt>{label}</dt>
      <dd>
        {flag && <span className="flag" title={flag === "≈" ? "Approximate: a level's max EXP was unknown" : "Some meso lines may have been missed in a burst"}>{flag}</span>}
        {value}
      </dd>
    </div>
  );
}

/** `recent` is the rolling last-5-minute rate, shown beside the session rate. */
function Sub({ label, value, recent }: { label: string; value: string; recent?: number }) {
  return (
    <div className="row sub">
      <dt>{label}</dt>
      <dd>
        {recent !== undefined && <span className="recent" title="Last 5 minutes">5m {formatInt(recent)} · </span>}
        {value}
      </dd>
    </div>
  );
}
