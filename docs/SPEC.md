# Battle Analysis: Spec

A lightweight desktop app that floats beside MapleStory Classic World and tracks EXP and mesos gained per minute during a recording session, like the in-game "Battle Analysis" window. It works **only** by reading pixels from the game window.

Sections are self-contained; read only the one a task needs. Decisions made after the original brief are marked **Decided**.

## 1. Hard constraints (never violate)

- Read-only screen capture of the game window. No game memory reading, no process injection, no input automation of any kind.
- Never save frames to disk, except an optional, off-by-default debug mode that saves only the small cropped regions (never full frames) to a local `debug-captures/` folder.
- No network calls. All data stays local. (Fonts are bundled, not loaded from Google Fonts.)

## 2. Stack and platform

- Tauri v2, Rust backend, React + TypeScript + Vite frontend.
- macOS first (native Nexon Launcher client). Windows later, so design for it now:
  - All OS-specific code goes behind Rust traits: `trait FrameSource` (find the game window, get the latest frame or a sub-rectangle of it) and the floating-window behavior (`trait FloatingWindow`).
  - Implement only the macOS versions now. Capture uses ScreenCaptureKit via the `screencapturekit` crate. **Decided:** chosen over `scap` because it exposes per-window filters and `sourceRect` sub-rectangle capture.
  - Never hardcode Retina/2x. Display scale factor is a variable everywhere (Windows uses 125%/150%).
- The floating panel must stay visible over the game in a native macOS fullscreen Space: floating level, joins fullscreen Spaces, doesn't steal focus. **Decided:** `tauri-nspanel`.

**Decided (layout):** a Cargo workspace. Pure logic (`capture` trait and types, `parse`, `stats`, `calibration`) lives in `crates/battle-core` with no OS or I/O dependencies. OS implementations and the app shell live in `src-tauri/`, and the UI lives in `src/`.

## 3. Data sources

1. **EXP (source of truth): the text above the EXP bar**, e.g. `EXP. 440,834 [38.43%]`, all one white color on a solid dark background. Crop only that text line, not the bar. Ignore the `EXP.` label. Also read the level number (bottom-left, `LV. 59`).
2. **Mesos: the system notification stack in the bottom-right corner.** Lines read `You have gained mesos (+15)`. The newest line appears at the bottom; older lines move up and fade. This text sits directly over map art, so the background varies by map. Lines like `You received EXP (+78)` and `Bonus EXP for PARTY (+23)` also appear there. **Decided:** only meso lines are counted; EXP comes from the EXP bar text alone. Other lines are recognised by their opening words (amount not read) so they can anchor stack alignment.

## 4. Reading text: template matching, not general OCR

The game uses a fixed bitmap font. For each region: crop → binarize by brightness → segment characters by column gaps → match against learned templates.

- Calibration teaches the font: the user drags a box around the EXP text, then types what it currently shows (e.g. `440834 38.43`). Slice the glyphs and save them as templates at the user's actual scale. Same for punctuation (`,` `.` `[` `]` `%` `(` `+` `)`).
- For notifications, match the fixed prefix `You have gained mesos (+` as one whole-image template, then read only the digits. Binarization must handle varied map backgrounds and fading lines.
- The parser should also report each notification line's brightness ("freshness"). The stats engine uses it to tell a repeated amount apart from the same line seen again (see §6).
- Store calibration (window id/title, region rects relative to the game window, templates) as small local files in the app data dir.

## 5. Capture loop

- Capture only while recording: the EXP region at ~2 fps, the notification region at ~5 fps. Capture just the sub-rectangles when the API allows it.
- Reject a reading unless it's self-consistent and two consecutive reads agree. Skip unreadable frames (menus, tooltips, Cash Shop, loading) rather than treating them as zero.
- No game window → Start disabled, with "MapleStory not detected" shown. Window lost mid-session → auto-pause.

## 6. Stats engine

Pure Rust, no I/O, no clock (timestamps are passed in), heavily unit tested. Lives in `crates/battle-core/src/stats/`.

- **Start:** take a baseline reading (level, exp, percent).
- **EXP up, same level:** add the delta.
- **Level up:** add (old level max − old exp) + new exp. Prefer a static level→EXP table for Classic/pre-Big Bang if one can be sourced reliably; otherwise derive max as exp ÷ percent and note the rounding error (worst at low percent). **Decided:** derived for now, using the highest-percent reading seen at each level. A table can be plugged in once it's validated against real readings. A jump of several levels between reads adds the full max of each skipped level when known; otherwise it's flagged approximate.
- **EXP down, same level (death):** add nothing, re-baseline.
- **Level down:** treat as a misread and ignore.
- **Mesos:** count each new notification line exactly once. Each frame, read the stack as an ordered list of lines and align it with the previous frame's list (find the upward shift). Only lines new at the bottom are counted. Two identical consecutive lines (`+15`, `+15`) must count twice. **Decided:** a line can only fade, never brighten, so a brighter match means a different line. Lines already on screen when recording starts or resumes are not counted.
- **Pause:** **Decided:** EXP and mesos gained while paused are not counted; resuming re-baselines.
- **Outputs:** battle duration (excluding paused time), EXP acquired, EXP/min, mesos acquired, mesos/min, time to next level (remaining EXP ÷ current rate), and a rolling last-5-minute rate. **Decided:** time to next level uses the rolling rate once 5 minutes have passed, the session rate before that.
- **States:** Idle → Recording ⇄ Paused → Stopped (results frozen) → Start New resets. **Decided:** "Start New Session" resets and starts recording in one click. Idle shows the previous session's results.

## 7. UI

One compact panel, ~300px wide. **The mockups in `docs/design/*.dc.html` are the source of truth for the look; their inline styles win over this section.** Use original artwork only, no Nexon assets.

- **Panel:** dark slate `#2b3139`, 2px `#8c939c` border, 12px radius, subtle drop shadow. Font: Nunito (bold/black weights), bundled locally.
- **Header:** `BATTLE ANALYSIS` in white, heavy weight. Small square buttons: `?` (gray), settings/calibration gear (green `#6fa52e`, Idle only per the mockups), close `×` (orange `#e07a24`).
- **Stats box:** darker inset `#1c2127`. The top line shows level and percent with a thin green progress bar. Rows: Battle Duration, EXP Acquired (with a "per min" sub-row), Mesos Acquired (with a "per min" sub-row), Next Level. Labels white, main values yellow `#ffd23f`, sub-rows muted gray.

| State | Mockup | Specifics |
| --- | --- | --- |
| **Idle** | `Main.dc.html` | Shows the last session. Big green pill button "Start Analyzing" (`#79b534`). |
| **Recording** | `Recording.dc.html` | Red `REC` pill in the header. Round gray Pause button + big orange-red "Stop" button (`#e0662a`). |
| **Stopped** | `Stopped.dc.html` | Green `RESULTS` pill. Shows the level range (e.g. Lv. 59 → 60) and the paused time excluded. Green "Start New Session" button. |

**Decided:** Paused reuses the Recording layout, with a gray `PAUSED` pill and a Resume button. A `MOCK DATA` badge shows whenever the numbers come from the synthetic feed rather than the game.

## 8. Testing

- `fixtures/` holds real game screenshots (committed PNG crops) plus expected values for each, as JSON. See `fixtures/README.md` for the format.
- Parser tests run against fixtures with no game running.
- Stats engine tests use synthetic reading sequences: steady kills, level-up, death, pause, unreadable frames, duplicate meso lines, and bursts of 4+ meso lines between frames.

### Test footage

There's a gameplay recording at `fixtures/video/grinding-01.mov`. ffmpeg is installed. `fixtures/video/` and `fixtures/frames/` are gitignored and must never be committed. Not used in Milestone 1. From Milestone 2:

- Extract frames at 5 fps into `fixtures/frames/grinding-01/`.
- Pick ~10 representative frames, show the user each one's EXP text crop and notification crop, and ask them to confirm the values before saving them as committed PNG fixtures with expected-values JSON.
- Use the full frame sequence to test meso line alignment: the total counted mesos must match the pickups in the video, including duplicate amounts in a row.

## 9. Milestones

Stop after each and summarize for review.

1. **Scaffold + core logic** ✅: Tauri v2 project, `.gitignore`, Rust module layout (`capture`, `parse`, `stats`, `calibration`), the full stats engine with unit tests, and the UI rendering all three states from mock data. No capture yet.
2. **Parser:** binarize/segment/template-match for the EXP text and notification lines, tested against fixtures and the test footage.
3. **Live capture + calibration:** macOS `FrameSource`, Screen Recording permission flow, window picker, region drag + glyph-teaching calibration.
4. **Floating panel:** always visible over the fullscreen game without stealing focus.
5. **Mesos end-to-end + polish:** stack alignment wired to live capture, debug-crop mode, sessions saved to local history (JSON).

## 10. README

The README covers: what it tracks; how it works (read-only capture, nothing saved or sent); install and the Screen Recording permission; calibration; the disclaimer *"Not affiliated with or endorsed by Nexon. Read-only screen capture; no game memory access or input automation. Use at your own risk."*; the roadmap (Windows); and dev setup. License: MIT.
