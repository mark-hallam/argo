# Archived briefs

Briefs in this directory are **not in the active queue.** They've been moved out of `streams/` to keep the workspace lean and the worker pool focused on v1.0 demo-gate work.

## Subdirectories

- **`shipped-by-other/`** — work already landed via a different PR. Verified by git log / code grep. Kept here for traceability; the original intent is already in main.
- **`stale-premise/`** — brief's described files/code no longer match current `origin/main`. Verified file-existence + symbol-grep. Premise is broken; the brief would need rewriting against current main to be revived.
- **`v1.x-deferred/`** — off-mission v1.1/1.2/1.4 polish briefs deferred until after the v1.0 demo MP4 ships. Strategic decision per operator + conductor brief-audit 2026-05-23.

## How to revive a brief

If a brief in this archive turns out to be useful again:

```bash
# 1. Move it back
git mv docs/build-plan/streams/archive/<subdir>/<brief-id>.md docs/build-plan/streams/

# 2. Validate the freshness header is still accurate
bash scripts/check-brief-freshness.sh <brief-id>

# 3. Add a row to docs/build-plan/BUILD_STATUS.md as TODO

# 4. Commit + push
git add docs/build-plan/streams/<brief-id>.md docs/build-plan/BUILD_STATUS.md
git commit -m "briefs: revive <brief-id> from archive"
```

## Audit cadence

This archive grows when:
- The conductor's tick-9 summary identifies stale/shipped briefs
- An operator-directed brief audit (like the 2026-05-23 one) classifies orphan briefs
- A worker discovers mid-claim that the brief's premise is broken

It shrinks rarely — most archived briefs stay archived. If you're considering reviving one, treat it as authoring a new brief: re-grep for the file paths, re-check the symbols, re-baseline the freshness SHA.

## Last large-scale archive sweep

**2026-05-23** — operator-directed sweep following the brief-audit at tick-66. ~410 aged orphan briefs (no ledger row, 7-30d old, authored before the 2026-05-14 v1-demo-gate strategy lock) classified by a dedicated audit agent. Moved per the agent's classification: shipped/stale/deferred buckets above.
