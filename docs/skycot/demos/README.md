# SKYCOT demos

Recorded product walkthroughs. Live artifacts are operator-recorded
and committed by the operator (Git LFS may apply); this directory
holds the scripts, checklists, and dry-run inputs that govern them.

## v1.0 mission demo

| File                                              | Purpose                                         |
| ------------------------------------------------- | ----------------------------------------------- |
| [`../demo_script.md`](../demo_script.md)          | Scene-by-scene recording script (5–7 min)       |
| [`recording-checklist.md`](recording-checklist.md)| Operator pre/during/post recording playbook     |
| [`../../../scripts/demo-dry-run.sh`](../../../scripts/demo-dry-run.sh) | Pre-recording validator (must be green) |
| `v1-mission-mvp.mp4`                              | The recorded artifact (operator-committed)      |

The recorded `.mp4` is **not** in the repo until the operator records
and commits it. The path is reserved so the marketing site and launch
docs can link it deterministically.

## Adding a new demo

1. Write the scene script as a sibling of `demo_script.md`.
2. Add a recording checklist sibling if the flow differs materially
   (different surface, different actor, different gates).
3. If the demo can be exercised end-to-end without operator input,
   add a dry-run sibling under `scripts/`. Otherwise reuse this one
   or document the manual gate.
4. Add a row to the table above.
