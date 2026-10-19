import { useCallback, useEffect, useRef, useState } from "react";
import { closeWindow, setup } from "../api";
import type { Image, Missing, RectPt, Regions, SetupStatus } from "../types";

type Target = "expText" | "level";

const targets: Record<Target, { label: string; hint: string; color: string }> = {
  expText: {
    label: "EXP numbers",
    hint: "Drag a box around the EXP text above the EXP bar. The EXP. label may be included; leave room on the right for the number to grow.",
    color: "#ffd23f",
  },
  level: { label: "Level", hint: "Drag a box around the level digits (bottom left, after LV.).", color: "#e07a24" },
};

/** Zoom for the region close-ups. */
const ZOOM = 3;

export function Calibrate() {
  const [status, setStatus] = useState<SetupStatus | null>(null);
  const [preview, setPreview] = useState<Image | null>(null);
  const [regions, setRegions] = useState<Partial<Regions>>({});
  const [target, setTarget] = useState<Target>("expText");
  const [values, setValues] = useState({ level: "", exp: "", pct: "" });
  const [missing, setMissing] = useState<Missing | null>(null);
  const [message, setMessage] = useState<{ ok: boolean; text: string } | null>(null);
  const [busy, setBusy] = useState(false);

  const fail = (e: unknown) => setMessage({ ok: false, text: String(e) });

  const applyStatus = useCallback((s: SetupStatus) => {
    setStatus(s);
    setMissing(s.missing);
    if (s.regions) setRegions(s.regions);
  }, []);

  const refreshStatus = () => setup.status().then(applyStatus);
  const refreshPreview = () => void setup.windowPreview().then(setPreview, fail);

  useEffect(() => {
    setup.status().then((s) => {
      applyStatus(s);
      if (s.game) setup.windowPreview().then(setPreview, fail);
    }, fail);
  }, [applyStatus]);

  if (status === null) return <div className="calib">Loading…</div>;

  if (status.permission === "denied") {
    return (
      <div className="calib">
        <h1>Calibration</h1>
        <section className="calib-step">
          <h2>Allow Screen Recording</h2>
          <p>
            Battle Analysis reads the EXP text from the game window&apos;s pixels. macOS needs your permission for
            that. Nothing is saved or sent anywhere.
          </p>
          <div className="calib-row">
            <button onClick={() => setup.requestPermission().then(() => refreshStatus(), fail)}>Ask for permission</button>
            <button onClick={() => setup.openPermissionSettings().catch(fail)}>Open System Settings</button>
            <button onClick={() => refreshStatus().catch(fail)}>Check again</button>
          </div>
          <p className="calib-muted">After allowing it in System Settings, quit and reopen Battle Analysis.</p>
        </section>
        {message && <Message {...message} />}
      </div>
    );
  }

  const chooseWindow = (id: number) => {
    setRegions({});
    setMissing(null);
    setup
      .chooseWindow(id)
      .then(() => refreshStatus())
      .then(refreshPreview, fail);
  };

  const teach = () => {
    const sample = parseSample(values);
    if (!regions.expText || !regions.level) return fail("Draw both boxes first.");
    if (typeof sample === "string") return fail(sample);
    setBusy(true);
    setMessage(null);
    setup
      .teach({ expText: regions.expText, level: regions.level }, sample)
      .then(async (m) => {
        setMissing(m);
        await refreshStatus();
        setMessage({ ok: true, text: "Learned. The boxes read back as the values you typed." });
      }, fail)
      .finally(() => setBusy(false));
  };

  const reset = () => {
    setup
      .reset()
      .then(() => {
        setRegions({});
        setMissing(null);
        setMessage(null);
        return refreshStatus();
      })
      .catch(fail);
  };

  return (
    <div className="calib">
      <h1>Calibration</h1>

      <section className="calib-step">
        <h2>1. Game window</h2>
        <div className="calib-row">
          <select value={status.game?.id ?? ""} onChange={(e) => chooseWindow(Number(e.target.value))}>
            {status.game === null && <option value="">Choose the MapleStory window…</option>}
            {status.windows.map((w) => (
              <option key={w.id} value={w.id}>
                {w.owner} — {w.title}
              </option>
            ))}
          </select>
          <button onClick={() => refreshStatus().catch(fail)}>Refresh list</button>
          {status.game && <span className="calib-muted">scale {status.game.scale}×</span>}
        </div>
      </section>

      {status.game && (
        <>
          <section className="calib-step">
            <h2>2. Mark the text</h2>
            <div className="calib-row">
              {(Object.keys(targets) as Target[]).map((t) => (
                <button key={t} className={t === target ? "selected" : ""} onClick={() => setTarget(t)}>
                  <i className="swatch" style={{ background: targets[t].color }} />
                  {targets[t].label}
                  {regions[t] ? " ✓" : ""}
                </button>
              ))}
              <button onClick={refreshPreview}>Refresh screenshot</button>
            </div>
            <p className="calib-muted">{targets[target].hint}</p>
            {preview && (
              <Preview image={preview} regions={regions} onDraw={(r) => setRegions((old) => ({ ...old, [target]: r }))} />
            )}
            <div className="calib-row">
              {(Object.keys(targets) as Target[]).map((t) => {
                const r = regions[t];
                return r ? <CloseUp key={t + JSON.stringify(r)} region={r} label={targets[t].label} /> : null;
              })}
            </div>
          </section>

          <section className="calib-step">
            <h2>3. Type what the game shows right now</h2>
            <div className="calib-row">
              <label>
                Level
                <input inputMode="numeric" placeholder="59" value={values.level} onChange={(e) => setValues({ ...values, level: e.target.value })} />
              </label>
              <label>
                EXP
                <input inputMode="numeric" placeholder="440,834" value={values.exp} onChange={(e) => setValues({ ...values, exp: e.target.value })} />
              </label>
              <label>
                Percent
                <input inputMode="decimal" placeholder="38.43" value={values.pct} onChange={(e) => setValues({ ...values, pct: e.target.value })} />
              </label>
              <button className="primary" disabled={busy} onClick={teach}>
                {busy ? "Learning…" : "Learn"}
              </button>
            </div>
            {missing && <MissingChars missing={missing} />}
          </section>
        </>
      )}

      {message && <Message {...message} />}

      <div className="calib-row calib-footer">
        <button onClick={reset}>Reset calibration</button>
        <button className="primary" onClick={() => void closeWindow()}>
          Done
        </button>
      </div>
    </div>
  );
}

function Message({ ok, text }: { ok: boolean; text: string }) {
  return <div className={ok ? "calib-ok" : "calib-error"}>{text}</div>;
}

function MissingChars({ missing }: { missing: Missing }) {
  if (!missing.exp && !missing.level) return <p className="calib-ok">Every character has been learned.</p>;
  return (
    <p className="calib-muted">
      Not learned yet:
      {missing.exp && <> EXP <code>{missing.exp.split("").join(" ")}</code></>}
      {missing.level && <> · Level <code>{missing.level.split("").join(" ")}</code></>}
      <br />
      Readings that use these will be skipped. Learn again after a few kills to teach more digits.
    </p>
  );
}

/** Parses the typed values; a string is an error message. */
function parseSample(v: { level: string; exp: string; pct: string }) {
  const level = Number(v.level.trim());
  const exp = Number(v.exp.replace(/[,\s]/g, ""));
  const pct = v.pct.replace(/[%\s[\]]/g, "");
  if (!Number.isInteger(level) || level < 1) return "Type the level, e.g. 59.";
  if (!/^\d+$/.test(v.exp.replace(/[,\s]/g, "")) || !Number.isSafeInteger(exp)) return "Type the EXP number, e.g. 440,834.";
  if (!/^\d{1,2}\.\d{2}$/.test(pct)) return "Type the percent with two decimals, e.g. 38.43.";
  const [whole, frac] = pct.split(".");
  return { level, exp, pctHundredths: Number(whole) * 100 + Number(frac) };
}

/** The window screenshot (1 px per point) with drag-to-draw boxes. */
function Preview({ image, regions, onDraw }: { image: Image; regions: Partial<Regions>; onDraw: (r: RectPt) => void }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const [drag, setDrag] = useState<{ x0: number; y0: number; x1: number; y1: number } | null>(null);

  useEffect(() => {
    const ctx = canvas.current?.getContext("2d");
    if (!ctx) return;
    ctx.putImageData(new ImageData(new Uint8ClampedArray(image.rgba), image.width, image.height), 0, 0);
    ctx.lineWidth = 1;
    for (const t of Object.keys(targets) as Target[]) {
      const r = regions[t];
      if (!r) continue;
      ctx.strokeStyle = targets[t].color;
      ctx.strokeRect(r.x + 0.5, r.y + 0.5, r.w - 1, r.h - 1);
    }
    if (drag) {
      ctx.strokeStyle = "#fff";
      ctx.setLineDash([3, 2]);
      const r = toRect(drag);
      ctx.strokeRect(r.x + 0.5, r.y + 0.5, r.w - 1, r.h - 1);
      ctx.setLineDash([]);
    }
  }, [image, regions, drag]);

  // Mouse position in canvas pixels, which are window points.
  const point = (e: React.PointerEvent<HTMLCanvasElement>) => {
    const b = e.currentTarget.getBoundingClientRect();
    return {
      x: Math.round(((e.clientX - b.left) / b.width) * image.width),
      y: Math.round(((e.clientY - b.top) / b.height) * image.height),
    };
  };

  return (
    <canvas
      ref={canvas}
      className="calib-preview"
      width={image.width}
      height={image.height}
      onPointerDown={(e) => {
        e.currentTarget.setPointerCapture(e.pointerId);
        const p = point(e);
        setDrag({ x0: p.x, y0: p.y, x1: p.x, y1: p.y });
      }}
      onPointerMove={(e) => drag && setDrag({ ...drag, x1: point(e).x, y1: point(e).y })}
      onPointerUp={() => {
        if (drag) {
          const r = toRect(drag);
          if (r.w >= 4 && r.h >= 4) onDraw(r);
        }
        setDrag(null);
      }}
    />
  );
}

function toRect(d: { x0: number; y0: number; x1: number; y1: number }): RectPt {
  return { x: Math.min(d.x0, d.x1), y: Math.min(d.y0, d.y1), w: Math.abs(d.x1 - d.x0), h: Math.abs(d.y1 - d.y0) };
}

/** A region at full resolution, zoomed, so the user can check the box covers the text. */
function CloseUp({ region, label }: { region: RectPt; label: string }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const [image, setImage] = useState<Image | null>(null);

  useEffect(() => {
    setup.regionPreview(region).then(setImage, () => setImage(null));
  }, [region]);

  useEffect(() => {
    const ctx = canvas.current?.getContext("2d");
    if (ctx && image) ctx.putImageData(new ImageData(new Uint8ClampedArray(image.rgba), image.width, image.height), 0, 0);
  }, [image]);

  return (
    <figure className="calib-closeup">
      {image && (
        <canvas ref={canvas} width={image.width} height={image.height} style={{ width: image.width * ZOOM, height: image.height * ZOOM }} />
      )}
      <figcaption>{label}</figcaption>
    </figure>
  );
}
