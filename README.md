# Battle Analysis

A small floating panel for **MapleStory Classic World** that tracks how much EXP and how many mesos you earn per minute while grinding, in the spirit of the in-game *Battle Analysis* window.

> **Not affiliated with or endorsed by Nexon. Read-only screen capture; no game memory access or input automation. Use at your own risk.**

> **Status:** early development. Milestone 1 (stats engine + UI on mock data) is done; live screen capture arrives in Milestone 3. See [Roadmap](#roadmap).

## What it tracks

| Stat | How |
| --- | --- |
| Battle duration | Recording time, excluding pauses |
| EXP acquired, EXP / min | From the `EXP. 440,834 [38.43%]` line above the EXP bar, plus the level |
| Mesos acquired, mesos / min | From `You have gained mesos (+15)` lines in the bottom-right notification stack |
| Last-5-minute rates | Rolling window over active time |
| Time to next level | Remaining EXP ÷ current rate |

The results view shows your level range (e.g. Lv. 59 → 60) and how much paused time was excluded.

## How it works

Battle Analysis reads **only the pixels of the game window**, the same way a screen recorder does:

1. While you're recording, it captures two small regions of the game window: the EXP text (about 2×/s) and the notification area (about 5×/s).
2. The game draws text in a fixed bitmap font, so instead of general OCR each region is binarized, split into characters, and matched against glyph templates learned from *your* screen during calibration.
3. A reading only counts when it's self-consistent (EXP agrees with percent) and two consecutive frames agree. Frames that can't be read, such as menus, tooltips, the Cash Shop or loading screens, are skipped rather than treated as zero.
4. Leveling up adds the rest of the old level plus the EXP into the new one. Dying (EXP going down) adds nothing. Each meso notification is counted once, including several identical `+15` lines in a row.

**Privacy:** Frames are processed in memory and discarded. Nothing is saved to disk except your calibration, plus small cropped text regions if you turn on the off-by-default debug mode. The app makes **no network connections**: fonts are bundled and everything stays on your machine.

**What it never does:** read or write game memory, inject into the game process, or send any keyboard/mouse input.

## Install

> Pre-built releases are not available yet. Build from source (see [Development](#development)).

### Screen Recording permission (macOS)

macOS requires permission before any app can capture another app's window:

1. Start Battle Analysis and click **Start Analyzing**. macOS will prompt for Screen Recording access.
2. Open **System Settings → Privacy & Security → Screen Recording** and enable **Battle Analysis**.
3. Quit and reopen the app.

## Calibration

*(Coming in Milestone 3.)* Calibration teaches the app the game's font at your resolution and display scale:

1. Pick the MapleStory window.
2. Drag a box around the EXP text above the EXP bar and type what it shows, e.g. `440834 38.43`. The app slices out each character and saves it as a template.
3. Do the same for the level number and the notification area.

Calibration is saved locally in the app's data directory. Redo it if you change resolution, window size or display scaling.

## Roadmap

- [x] **M1**: Project scaffold, stats engine with tests, UI on mock data
- [ ] **M2**: Bitmap-font parser for EXP and notification text, tested against real screenshots
- [ ] **M3**: Live macOS capture (ScreenCaptureKit), permission flow, window picker, calibration
- [ ] **M4**: Floating panel that stays above the fullscreen game without stealing focus
- [ ] **M5**: Mesos end-to-end, debug-crop mode, local session history
- [ ] **Windows** support (Windows.Graphics.Capture). Capture and floating-window code already sit behind traits, and display scale (125% / 150%) is never hardcoded.
- [ ] Static level → EXP table for Classic World (currently each level's max EXP is derived from EXP ÷ percent; see below)

### Accuracy note: level-ups

Until a verified level table is bundled, the EXP needed for a level is estimated as `exp ÷ percent` from the highest-percent reading seen at that level. The percent has two decimals, so the estimate is off by roughly `0.005 / percent`: about ±0.01% when based on a 38% reading, but up to ±0.5% when based on a 1% reading. This only affects the "rest of the old level" part of a level-up.

## Development

Prerequisites: macOS with Xcode Command Line Tools, [Rust](https://rustup.rs) (stable), Node.js 20+.

```sh
npm install
npm run tauri dev        # run the app (drives the real stats engine from a mock feed in M1)
npm run dev              # UI only, in a browser, with mock data; ?mock=idle|recording|paused|stopped|empty
cargo test -p battle-core   # stats engine + core tests
cargo clippy --workspace --all-targets -- -D warnings
```

### Layout

```
crates/battle-core/   Pure Rust, no OS or I/O: stats engine, parser, calibration types, FrameSource trait
src-tauri/            Tauri app: commands, OS-specific capture and floating-window implementations
src/                  React + TypeScript panel
fixtures/             Committed test screenshots + expected values (video/ and frames/ are gitignored)
```

## License

[MIT](LICENSE)
