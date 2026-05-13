# claude-rebase-pr-1511 — recovery brief for stalled #1511 (worker settings + danger-zone endpoints (J11))

**Owner**: claude. **Depends on**: nothing (recovery refactor).
**Authored against**: origin/main `902d9bbc` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-rebase-pr-1511`.

## Scope

Recovery for PR #1511 (`claude-J11-gateway-worker-settings-endpoints`). ~41h DIRTY against current `origin/main`. Original work is fine in shape (+1243/-0 10 files); just needs a rebase + conflict resolution + force-push.

**Gateway-zone collision PR — SECOND in J-series sequence (rebase AFTER #1504 J8 lands).** Touches all 4 `routes_dispatch_{delete,get,patch,post}.rs` files plus `mod.rs`. Rebase against current main (HEAD `902d9bbc`) which now includes #1566 + boot.rs split (#1544). If #1504 is still DIRTY at claim time, BLOCK this brief and let #1504 land first to avoid compounding the gateway/mod.rs conflicts.

**Specific actions**:

1. **Chain-rebase against current `main` (HEAD `902d9bbc`)** — many landings since the PR's last push (boot.rs split #1544, runbook split #1545, migrations-mod split #1550, mcp-pool #1540, structured-logging #1555, error-types Schema rewrite #1566, roster policy_override #1567, plus other cleanup PRs). May-touch zones: `crates/argo-cli/src/handlers/gateway/mod.rs`, `routes_dispatch_{delete,get,patch,post}.rs`, NEW `routes_worker_settings.rs` + `_danger.rs` + `_helpers.rs` + tests..

2. **Resolve additive conflicts** — take both sides on Cargo.lock, BUILD_STATUS.md, lib.rs re-exports, config-section field additions.

3. **Pattern updates from session merges**:
   - If brief touched `migrations/mod.rs` registry → use `migrations/registry.rs` (post-#1550 split).
   - If brief touched boot.rs spawn helpers → use `boot_spawns.rs` (post-#1544 split).
   - If brief appended to RUNBOOK.md → use per-topic sub-file under `docs/skycot/runbook/` (post-#1545 split).
   - If brief raised `ArgoError::Provider(format!("invalid|missing|bad|unknown ..."))` for caller-supplied input → switch to `ArgoError::Schema(...)` (post-#1566 sweep).

**Not in scope**: changing the original design; introducing new features; widening may-touch beyond the original PR's footprint.

## Files you may touch

- Whatever PR #1511 already touched (`gh pr diff 1511 --color never | head -300`).
- Resolve additive conflicts as listed above.

## Files you must NOT touch

- Anything outside PR #1511's original may-touch set.
- The original design — this is rebase-only.

## Acceptance criteria

- [ ] `gh pr checkout 1511 && git fetch origin main && git merge origin/main`.
- [ ] All conflicts resolved.
- [ ] If a migration slot needs renumbering (check `ls crates/argo-store/migrations/` for free slot), do so and avoid v60.rs.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `git push --force-with-lease origin claude-J11-gateway-worker-settings-endpoints`.
- [ ] PR #1511 mergeStateStatus returns to MERGEABLE.

## Branch + PR

- Branch: `claude-J11-gateway-worker-settings-endpoints` (existing).
- PR title: existing.

## Notes for the implementer

- ~10-30 min mechanical rebase depending on conflict surface.
- `--force-with-lease` only.
- Conductor-authored 2026-05-14 batch; one of 6 recovery briefs in this wave.
