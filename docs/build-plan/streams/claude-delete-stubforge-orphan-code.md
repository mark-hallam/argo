# claude-delete-stubforge-orphan-code — keep trait+types; delete orphan StubForge + un-wired persistence

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `6e3e31d8` on `2026-05-14`.

> **Phase 3 wire-or-delete decision (CEO-authorized 2026-05-14): SHRINK.** Audit cluster 3 O-1 found `argo-forge` is ~1800 LOC with `StubForge` the only impl. `optimize` returns "Python plugin not wired"; `cost::*`, `persist::*`, `feedback_eval::*` only called from tests. Real implementation depends on `argo-plugin-py:dspy-gepa` which isn't in scope this quarter.

## Scope

Delete the orphan code paths; keep the `Forge` trait + `ForgeRun`/`ForgeOptions`/`ForgeEvent` types in `argo-types` as a future seam. Add a `docs/skycot/runbook/forge-deferred.md` documenting the v1.x deferral.

Specifically:
- Delete `StubForge` and `optimize` stub return.
- Delete `cost::*` (only tests call it).
- Delete `persist::*` (only tests).
- Delete `feedback_eval::*` (only tests).
- Delete `spawn_forge_cron` (enqueues rows nothing dequeues).
- KEEP `Forge` trait + run/options/event types in argo-types.
- KEEP the `forge_run` SQL table (no production writer means it's empty; safe).

## Files you may touch
- `crates/argo-forge/src/forge.rs` — delete StubForge.
- `crates/argo-forge/src/cost.rs`, `persist.rs`, `feedback_eval.rs`, `cron.rs` — `git rm`.
- `crates/argo-forge/src/lib.rs` — narrowly: prune re-exports.
- `crates/argo-cli/src/handlers/forge_*.rs` — narrowly: replace bodies with `eprintln!("forge deferred to v1.x; see runbook/forge-deferred.md")` + exit 78.
- `<NEW: docs/skycot/runbook/forge-deferred.md>` (≤100 LOC) — explains the deferral, future Python plugin path.

## Files you must NOT touch
- `argo-types` Forge trait + types — KEEP.
- `forge_run` SQL table or its migration — KEEP.

## Acceptance
- [ ] Net LOC ~-1500 (keep ~300 LOC of trait+types).
- [ ] `argo forge <subcommand>` returns the deferred message; doesn't crash.
- [ ] `cargo build --workspace` clean.
- [ ] Workspace clippy clean.

## Branch
- `claude-delete-stubforge-orphan-code`
