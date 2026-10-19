# Fixtures

Real game screenshots used by the parser tests (Milestone 2). Tests run with no game running.

- `*.png`: committed crops of real frames (EXP text region, notification region).
- `*.json`: expected values for the PNG with the same name, for example:

```json
{ "level": 59, "exp": 440834, "pctHundredths": 3843 }
```

```json
{ "lines": [{ "kind": "meso", "amount": 15 }, { "kind": "exp", "amount": 78 }] }
```

**Never committed** (gitignored): `video/` (gameplay recordings) and `frames/` (frames extracted from them).
