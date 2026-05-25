# Stream ops-cargo-cache-eviction-policy-when-over-150gb — auto-evict shared cargo target dir when over 150GB

**Owner**: claude (bash/scripting + cron entry; ~60-100 LOC).
**Depends on**: none.
**Authored against**: origin/main `be9206ef2` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-cargo-cache-eviction-policy-when-over-150gb`.

🟡 **P2 ops-hygiene.** Source: operator interactive session 2026-05-24T23:30Z. The silent-fail worker on `codex-cli-routes-approvals-json-split-helpers` reported `"Build cache is huge (321GB, 836k files) — explains the slow clippy"` after iter 2, then spent another 4 hours waiting on clippy that never finished due to cross-worker contention. The root cause is the shared cargo target dir has no eviction policy — every worker iteration adds artifacts, nothing cleans up.

## Scope

Add a periodic eviction script + cron entry that:

1. **Measures**: `du -sk <CARGO_TARGET_DIR>` for the shared target dir paths used by workers (see `claude-worker-loop.sh` for the canonical list — likely `/tmp/argo-shared-target` + per-brief `/tmp/<brief>-target` dirs).
2. **Threshold check**: if total cache > 150GB (configurable env `ARGO_CACHE_EVICTION_GB_THRESHOLD`, defaults 150), trigger eviction.
3. **Eviction strategy**: oldest-first by `mtime` on the brief-specific target dirs. Keep the most-recent 5 brief dirs even if over threshold (workers in flight). Never touch `~/.cargo/registry/` (cross-iteration cache, not workspace artifacts).
4. **Lockfile**: respect a `/tmp/argo-cache-eviction.lock` so two eviction runs don't race + no worker iteration starts mid-eviction.
5. **Cron / scheduled-task**: every 30min via a scheduled-task entry (analogous to the worker-supervisor-watchdog cron).

The 321GB observation is real, the 150GB threshold is a starting heuristic — operator can tune via env.

## Grep before write

```
rg -n "CARGO_TARGET_DIR|/tmp/argo-shared-target|/tmp/<.*>-target" scripts/ ~/.claude/scheduled-tasks/
```

Locate where worker target dirs are configured. Confirm whether `~/.argo/` has any related dir as well (if so, scope it in).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: [worker shared cargo dirs]

## Files you may touch

- `scripts/argo-cache-eviction.sh` (NEW, ~50-80 LOC) — the eviction script.
- `scripts/argo-cache-eviction-tests.sh` (NEW) — bash smoke harness with 3 cases: (a) under threshold → no-op; (b) over threshold → evict oldest-first, preserve newest 5; (c) lockfile present → no-op + log skip.
- `~/.claude/scheduled-tasks/skycot-cache-eviction-30min/SKILL.md` (NEW, ~20 LOC) — scheduled-task entry. (This is operator-owned outside the repo; the brief should ship the repo-side script + reference the operator-step to install the schedule.)

## Tests

- Smoke harness with 3 synthetic /tmp dirs at varying mtime + sizes.
- Manual smoke: dry-run mode (`--dry-run`) that prints what would be evicted without doing it. Should be the default for the first commit.

## Definition of done

- Script exists, dry-run mode produces sensible eviction list against current `/tmp/`.
- 3 smoke cases green.
- PR body explicitly documents the operator-step to install the scheduled-task entry.
- Threshold + retention env vars documented in script header.

Notes: do NOT touch `~/.cargo/registry/` (registry download cache is shared cross-iteration and expensive to re-fetch). Only target dirs (compiled artifacts). Conservative defaults — start with dry-run-as-default for the first PR; flip to enforcing in a follow-up after operator confirms behavior.
