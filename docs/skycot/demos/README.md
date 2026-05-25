# SKYCOT demos

Recorded product walkthroughs. Live artifacts are operator-recorded
and committed by the operator (Git LFS may apply); this directory
holds the scripts, checklists, and dry-run inputs that govern them.

## v1.0 mission demo — canonical

| File                                                              | Purpose                                                              |
| ----------------------------------------------------------------- | -------------------------------------------------------------------- |
| [`v1-mission-mvp-runbook.md`](v1-mission-mvp-runbook.md)          | **Canonical** scene-by-scene runbook — 6 scenes, one per MISSION.md claim, 8-minute target, 10-minute hard cap |
| [`../../../scripts/demo-dry-run.sh`](../../../scripts/demo-dry-run.sh) | Pre-recording validator (must be green; use `--demo-mode` for recording sessions) |
| [`../runbook/v1-demo-recording.md`](../runbook/v1-demo-recording.md)  | Operator machine setup — Slack app, tunnel, env vars, SQLCipher key, demo-mode env vars |
| `v1-mission-mvp.mp4`                                              | The recorded artifact (operator-committed; not in repo until recording lands) |

The recorded `.mp4` is **not** in the repo until the operator records
and commits it. The path is reserved so the marketing site and launch
docs can link it deterministically.

## v1.0 mission demo — historical-archived

The May-17 pair below pre-dates the six-claim mission structure and
the demo-mode auth bypass (#2164). Kept as source material for
talk-track lines that were folded into the canonical runbook; not the
script to narrate against.

| File                                              | Status                              |
| ------------------------------------------------- | ----------------------------------- |
| [`../demo_script.md`](../demo_script.md)          | `historical-archived` — single-vertical 5–7 min happy path |
| [`recording-checklist.md`](recording-checklist.md)| `historical-archived` — pre/during/post checklist for the May-17 script |
| [`../operator/demo-runbook.md`](../operator/demo-runbook.md) | `historical-archived` — older 8-min voiceover with stale routes |

## Adding a new demo

1. Write the scene runbook in `demos/` (one canonical doc per demo,
   six-scene-or-similar structure).
2. Add a dry-run sibling under `scripts/` if the demo can be exercised
   end-to-end without operator input.
3. Cross-link from the canonical runbook to the operator machine
   setup in `runbook/`, not the other way around.
4. Add a row to the canonical table above.
