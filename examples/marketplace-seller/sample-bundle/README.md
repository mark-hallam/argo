# Sample artifact bundle

Mirrors the layout that lands at `.skycot/outputs/<job_id>/`
after running:

```bash
argo work add --vertical gtm-engineer --prompt "$(cat ../sample-prompts.md | …skates section)"
argo work serve --vertical gtm-engineer --once
```

## Files in this directory

| File                       | What it is                                                      |
|----------------------------|-----------------------------------------------------------------|
| `transcript.md`            | Full user/assistant turn log.                                  |
| `assistant_output.md`      | The worker's final assistant text in full.                     |
| `research.md`              | Cited research the worker pulled before drafting.              |
| `pricing.md`               | Comparison table + pricing recommendation.                     |
| `listings.md`              | Draft listing copy for each platform.                          |
| `approval_checklist.md`    | The operator's go/no-go list. Carries BLOCKED banner if any.   |
| `execution_plan.md`        | Action plan grouped by state.                                  |
| `actions.json`             | Machine-readable mirror of `execution_plan.md`.                |
| `job.json`                 | Metadata.                                                       |
| `result.json`              | Index pointers + counts.                                        |
| `actions/<action_id>/instructions.md` | Per-action operator brief (rendered by the human executor at execute time). |
| `actions/<action_id>/evidence.md`     | Per-action cited sources.                              |
| `actions/<action_id>/action.json`     | Per-action full snapshot.                              |

## Notes on the sample data

This sample is hand-crafted to match a successful worker run
on the `skates` prompt with sufficient research evidence — i.e.
a happy-path day. Files `actions.json` + `result.json` use the
schema your real run will produce; the markdown files are
written in the style the system prompt steers the worker
toward.

For the BLOCKED variant (worker emitted listings on top of weak
research), see `examples/marketplace-seller/sample-bundle/
approval_checklist_blocked_variant.md` (TODO — for future PR).
