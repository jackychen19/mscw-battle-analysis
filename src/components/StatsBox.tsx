import type { Snapshot } from "../types";
import { formatDuration, formatEta, formatInt, formatPct, formatShortDuration } from "../format";

/** Meso tracking is on hold (SPEC §9), so its rows are hidden. */
const SHOW_MESOS = false;

/** `placeholder` renders dashes for values (idle with no previous session). */
export function StatsBox({ snap, caption, placeholder = false }: { snap: Snapshot; caption: string; placeholder?: boolean }) {
  const dash = (v: string) => (placeholder ? "—" : v);
  const stopped = snap.state === "stopped";
  const pct = placeholder ? 0 : (snap.pctHundredths ?? 0);
  const levelUp = snap.startLevel !== null && snap.level !== null && snap.level > snap.startLevel;
  const levelText =
    placeholder || snap.level === null
      ? "Lv. — · —"
      : stopped && levelUp
        ? `Lv. ${snap.startLevel} → Lv. ${snap.level}`
        : `Lv. ${snap.level} · ${formatPct(snap.pctHundredths)}`;

  return (
    <section className="stats-box">
      <div className="caption-line">
        <span>{caption}</span>
        <span>{levelText}</span>
      </div>
      <div className="progress" role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={pct / 100}>
        <div className="progress-fill" style={{ width: `${pct / 100}%` }} />
      </div>
      <div className="progress-gap" />

      <dl className="rows">
        <Row label="Battle Duration" value={dash(formatDuration(snap.durationMs))} />
        <Row label="EXP Acquired" value={dash(formatInt(snap.expGained))} flag={snap.expApproximate ? "≈" : undefined} />
        <Row sub label="per min" value={dash(`${formatInt(snap.expPerMin)} / min`)} />
        {SHOW_MESOS && (
          <>
            <Row label="Mesos Acquired" value={dash(formatInt(snap.mesoGained))} flag={snap.mesoPossibleUndercount ? "≥" : undefined} />
            <Row sub label="per min" value={dash(`${formatInt(snap.mesoPerMin)} / min`)} />
          </>
        )}
        {stopped ? (
          <Row muted label="Paused" value={`${formatShortDuration(snap.pausedMs)} (not counted)`} />
        ) : (
          <Row label="Next Level" value={dash(formatEta(snap.nextLevelMs))} />
        )}
      </dl>
    </section>
  );
}

const flagTitles: Record<string, string> = {
  "≈": "Approximate: a level's max EXP was unknown",
  "≥": "Some meso lines may have been missed in a burst",
};

function Row({ label, value, flag, sub = false, muted = false }: { label: string; value: string; flag?: string; sub?: boolean; muted?: boolean }) {
  return (
    <div className={`row${sub ? " sub" : ""}${muted ? " muted" : ""}`}>
      <dt>{label}</dt>
      <dd>
        {flag && <span className="flag" title={flagTitles[flag]}>{flag}</span>}
        {value}
      </dd>
    </div>
  );
}
