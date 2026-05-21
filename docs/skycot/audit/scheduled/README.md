# Scheduled audit reports

This directory contains reports from the SKYCOT scheduled auditor (charter: [`../AUDITOR_CHARTER.md`](../AUDITOR_CHARTER.md)).

## Layout

- `LATEST.md` — most recent audit report (overwritten each run).
- `<UTC-YYYY-MM-DDTHH-MM>.md` — timestamped immutable report per run.

## Convention

- Reports are committed by the auditor session with commit message: `audit(scheduled): <UTC timestamp> — <N> recommendations, orientation Δ<±score>`.
- The conductor reads `LATEST.md` at the top of every hourly tick (Step 4.5 in `docs/build-plan/PROTOCOL_LOOP_PROMPT.md`).
- The operator reads any timestamped report on demand: `open docs/skycot/audit/scheduled/LATEST.md`.

## Inspecting the most-recent report

```bash
open ~/code/argo/docs/skycot/audit/scheduled/LATEST.md
```

## History

```bash
ls docs/skycot/audit/scheduled/*.md | sort -r
```
