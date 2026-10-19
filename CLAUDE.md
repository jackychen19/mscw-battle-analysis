# Battle Analysis

Floating EXP/meso rate tracker for MapleStory Classic World. Reads pixels from the game window only. Full spec: `docs/SPEC.md` — read the relevant section only when a task needs it, not by default.

## Hard rules
- Read-only screen capture. Never read game memory, inject, or automate input.
- Never write full frames to disk. Debug mode saves small crops to `debug-captures/` only.
- No network calls.

## Stack
Tauri v2 · Rust backend · React + TS + Vite frontend. macOS first; Windows later.
- OS-specific code only behind traits (`FrameSource`, floating window). macOS impls only for now.
- Never hardcode Retina/2x; scale factor is always a variable.

## Layout
<!-- Claude: correct these paths to match the actual project -->
Pure logic lives in the `battle-core` crate (no OS, no I/O); OS-specific code lives in the Tauri app.
- `crates/battle-core/src/capture/` — FrameSource trait + scale-aware rect/frame types
- `crates/battle-core/src/parse/` — binarize, segment, template-match (EXP text, meso notifications)
- `crates/battle-core/src/stats/` — pure stats engine, no I/O; scenario tests in `stats/tests/`
- `crates/battle-core/src/calibration/` — regions + glyph templates (file format)
- `src-tauri/src/` — Tauri app: commands, mock feed (M1), floating window trait; macOS FrameSource impl goes here
- `src/` — panel UI (Idle / Recording / Stopped); browser mock in `src/mock/`
- `fixtures/` — committed PNGs + expected JSON
- `docs/design/*.dc.html` — UI mockups; their inline styles are the source of truth for the panel's look

## Commands
<!-- Claude: verify and correct -->
- Dev: `npm run tauri dev` — always run in the background, never as a blocking command
- Rust tests: `cargo test`
- Frontend checks: `npm run lint && npm run typecheck`

## Key logic (don't re-derive)
- EXP source of truth = white text above the EXP bar (`440,834 [38.43%]`), not the bar itself.
- Mesos = bottom-right notification stack, `You have gained mesos (+N)`. Count new lines once via frame-to-frame stack alignment; identical consecutive lines count twice.
- EXP down at same level = death → add nothing, re-baseline. Level up → (old max − old exp) + new exp.
- No game window → Start disabled, show "MapleStory not detected". Window lost mid-session → auto-pause.

## Working style
- Don't take screenshots to verify UI. Ask me to check it.
- Run `cargo test` for any change in `parse/` or `stats/` before saying it's done.
- Don't open or list `fixtures/video/`, `fixtures/frames/`, `target/`, `node_modules/`, or `debug-captures/`. Ask before viewing images in bulk.
- Keep changes scoped to the current milestone. Ask before adding dependencies.

## Git
- Commit after each logical step (a module working with its tests, a UI state done), not one giant commit per milestone.
- Clear, short commit messages using Conventional Commits (`feat:`, `fix:`, `test:`, `docs:`, `chore:`).
- Never push or force-push without asking me.