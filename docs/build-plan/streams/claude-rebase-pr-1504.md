# claude-rebase-pr-1504 — recovery brief for stalled #1504 (GET /api/v1/workers/<id>/overview endpoint (J8))

**Owner**: claude. **Depends on**: nothing (recovery refactor).
**Authored against**: origin/main `902d9bbc` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-rebase-pr-1504`.

## Scope

Recovery for PR #1504 (`claude-J8-gateway-worker-overview-endpoint`). ~42h DIRTY against current `origin/main`. Original work is fine in shape (+899/-1 6 files); just needs a rebase + conflict resolution + force-push.

**Gateway-zone collision PR.** Rebase against current main (HEAD `902d9bbc`) which now includes #1566 (mechanical Provider→Schema variant rewrite — single line in `handlers/gateway/boot.rs`; verify your `routes_dispatch_get.rs` additions don't shadow the same callsite if any) plus the boot.rs split (#1544 → `boot_spawns.rs`). **This PR is the FIRST in the J-series sequence (J8 then J11 #1511). 4 ledger-BLOCKED PRs (#1494, #1482, #1469, #1503) + #1564 wait on this + #1511 merging.**

**Specific actions**:

1. **Chain-rebase against current `main` (HEAD `902d9bbc`)** — many landings since the PR's last push (boot.rs split #1544, runbook split #1545, migrations-mod split #1550, mcp-pool #1540, structured-logging #1555, error-types Schema rewrite #1566, roster policy_override #1567, plus other cleanup PRs). May-touch zones: `crates/argo-cli/src/handlers/gateway/mod.rs`, `crates/argo-cli/src/handlers/gateway/routes_dispatch_get.rs`, NEW `routes_worker_overview.rs` + tests..

2. **Resolve additive conflicts** — take both sides on Cargo.lock, BUILD_STATUS.md, lib.rs re-exports, config-section field additions.

3. **Pattern updates from session merges**:
   - If brief touched `migrations/mod.rs` registry → use `migrations/registry.rs` (post-#1550 split).
   - If brief touched boot.rs spawn helpers → use `boot_spawns.rs` (post-#1544 split).
   - If brief appended to RUNBOOK.md → use per-topic sub-file under `docs/skycot/runbook/` (post-#1545 split).
   - If brief raised `ArgoError::Provider(format!("invalid|missing|bad|unknown ..."))` for caller-supplied input → switch to `ArgoError::Schema(...)` (post-#1566 sweep).

**Not in scope**: changing the original design; introducing new features; widening may-touch beyond the original PR's footprint.

## Files you may touch

- Whatever PR #1504 already touched (`gh pr diff 1504 --color never | head -300`).
- Resolve additive conflicts as listed above.

## Files you must NOT touch

- Anything outside PR #1504's original may-touch set.
- The original design — this is rebase-only.

## Acceptance criteria

- [ ] `gh pr checkout 1504 && git fetch origin main && git merge origin/main`.
- [ ] All conflicts resolved.
- [ ] If a migration slot needs renumbering (check `ls crates/argo-store/migrations/` for free slot), do so and avoid v60.rs.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `git push --force-with-lease origin claude-J8-gateway-worker-overview-endpoint`.
- [ ] PR #1504 mergeStateStatus returns to MERGEABLE.

## Branch + PR

- Branch: `claude-J8-gateway-worker-overview-endpoint` (existing).
- PR title: existing.

## Notes for the implementer

- ~10-30 min mechanical rebase depending on conflict surface.
- `--force-with-lease` only.
- Conductor-authored 2026-05-14 batch; one of 6 recovery briefs in this wave.
