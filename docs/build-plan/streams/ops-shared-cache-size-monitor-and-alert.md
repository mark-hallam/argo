# ops-shared-cache-size-monitor-and-alert — alert before cargo cache hits >50 GB

**Owner**: claude or codex. **Depends on**: nothing.
**Authored against**: origin/main `289d3077f` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-shared-cache-size-monitor-and-alert`.

## Why this is P3

CLAUDE.md diagnostic finding (2026-05-25 merge train) documents the macOS jetsam-kill outage was rooted in workers' shared `/tmp/argo-shared-target` cache growing to **328 GB** while free RAM was pegged at ~30 MB. Fixed for now by moving cache to `$HOME/.cache/argo-shared-target/`, but the same growth can happen there. We need a watchdog signal BEFORE the cache becomes a memory-pressure event.

## Scope

1. Add a cache-size check to `scripts/worker-supervisor-watchdog.sh`:
   - Measure `$HOME/.cache/argo-shared-target/` total size with `du -sk` (or equivalent).
   - Buckets: <20 GB GREEN, 20-50 GB YELLOW (log line), >50 GB RED (SHARED.md alert + heartbeat warning).
   - Cap configurable via `CACHE_SIZE_YELLOW_GB` (default 20) + `CACHE_SIZE_RED_GB` (default 50).
2. Add a `scripts/prune-shared-cargo-cache.sh` that:
   - Removes anything in `$HOME/.cache/argo-shared-target/` older than 7 days (`find -mtime +7 -delete`).
   - Reports total pruned size.
   - Idempotent.
3. Document a recommended cadence in `docs/skycot/operator/cache-maintenance.md` (new): "run weekly via cron or manually".

## Files you may touch

- `scripts/worker-supervisor-watchdog.sh` — add cache-size check.
- `scripts/prune-shared-cargo-cache.sh` — new.
- `docs/skycot/operator/cache-maintenance.md` — new.

## Files you must NOT touch

- Anything under `crates/`.

## Acceptance criteria

- [ ] Watchdog reports cache size in its heartbeat line.
- [ ] At >50 GB, SHARED.md gets a `[watchdog-cache-pressure]` alert.
- [ ] Prune script removes >7-day-old entries; second run is a no-op.
- [ ] Runbook covers: when to prune, how to read the watchdog signal.

## Branch + PR

- Branch: `claude-ops-shared-cache-size-monitor-and-alert` or `codex-…`.
- PR title: `ops(watchdog): cargo cache size monitor + prune script (ops-shared-cache-size-monitor-and-alert)`.

## Notes

- ~70-120 LOC bash total.
- Authorized under `--skip-clippy` per evaluator-pre-verified ops-* path.
