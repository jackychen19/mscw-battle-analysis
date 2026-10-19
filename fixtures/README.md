# Fixtures

Real game footage used by the parser tests. Tests run with no game running.

## Committed: `<recording>/NNNN.*`

One set per recording, named after the frame number at 5 fps (frame N is at (N−1)/5 s):

- `NNNN.exp.png`, `NNNN.level.png`, `NNNN.notif.png`: crops of the EXP text, level digits and notification area.
- `NNNN.json`: expected values, confirmed by a person.

```json
{
  "source": "grinding-01",
  "frame": 465,
  "timeS": 92.8,
  "exp": { "level": 59, "exp": 422579, "pctHundredths": 3684 },
  "notifications": [
    { "kind": "meso", "amount": 15, "faded": true },
    { "kind": "exp", "amount": 8 },
    { "kind": "partyBonus", "amount": 2 },
    { "kind": "other", "text": "Jr. Necki Skin x1 earned. (Etc)" },
    { "kind": "unreadable", "note": "faded and covered by a monster" }
  ]
}
```

Notification lines are listed top to bottom.

`bar-0465.png` is the bottom status bar of frame 465 (video pixels 1260,1820 to 3400,1990), for finding the EXP text and level with no calibration. Its readings are the ones confirmed for `0465.json`. The streamer's overlay on the right shows a second LV/EXP readout, which the finder must ignore.

### grinding-01

A screen recording of a YouTube video of PixelParakeet's stream (Lv59 Assassin, Sleepy Dungeon), so the text is scaled by a non-integer factor and video-compressed: a harder case than a native capture. Only 0–122 s is used; after that the browser window moves.

### native-01

`bar.png`: the status bar cut from a macOS screenshot of the game window (Lv 33, EXP 117,832 [84.98%]). Unlike grinding-01 it's sharp, as a direct window capture is: each character is drawn identically.

## Never committed (gitignored)

- `video/`: gameplay recordings.
- `frames/`: crops extracted from them by `scripts/extract-crops.sh`. Tests that need these skip when they're missing.
