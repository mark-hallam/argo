# claude-rebase-pr-1463 — recovery brief for stalled #1463 (mission-ip-allowlist-deployment-mode)

**Owner**: claude. **Depends on**: nothing (recovery refactor).
**Authored against**: origin/main `062cd425` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-rebase-pr-1463`.

## Scope

Recovery for PR #1463 (`claude-mission-ip-allowlist-deployment-mode`). ~20h DIRTY since 2026-05-12T16:32Z. Recipe posted 18h ago, no rebase action. Per the PR-review agent's 2026-05-13T10:30Z report, this PR is past the 14h recovery threshold.

Touch boot.rs (now split via #1544 → use boot_spawns.rs for the spawn helper), Cargo.toml additive deps, RUNBOOK.md (split via #1545 → move section to runbook/ sub-file).

**Specific actions**:

1. **Chain-rebase against current `main` (HEAD `062cd425`)** — many landings since the PR's last push (boot.rs split #1544, runbook split #1545, migrations-mod split #1550, prompt-cache trio #1539/#1543/#1549, mcp-pool #1540, structured-logging #1555, plus other docs/cleanup PRs). May-touch zones: crates/argo-cli/src/handlers/gateway/ + argo-config.

2. **Resolve additive conflicts** — take both sides on RuntimeConfig field additions, lib.rs re-exports, Cargo.lock, BUILD_STATUS.md ledger rows.

3. **Pattern updates from session merges**:
   - If brief cited `sections_runtime.rs` → use `sections_context.rs` (the actual home of RuntimeConfig).
   - If brief touched `migrations/mod.rs` for migration registration → use `migrations/registry.rs` (post-#1550 split).
   - If brief appended to RUNBOOK.md → consider per-topic sub-file under `docs/skycot/runbook/` (post-#1545 split, RUNBOOK.md is 492 LOC).
   - If brief touched boot.rs spawn helpers → use `boot_spawns.rs` (post-#1544 split).

**Not in scope**: changing the original design; introducing new features; widening may-touch beyond the original PR's footprint.

## Files you may touch

- Whatever PR #1463 already touched (`gh pr diff 1463 --color never | head -300`).
- Resolve additive conflicts as listed above.

## Files you must NOT touch

- Anything outside PR #1463's original may-touch set.
- The original design — this is rebase-only.

## Acceptance criteria

- [ ] `gh pr checkout 1463 && git fetch origin main && git merge origin/main`.
- [ ] All conflicts resolved.
- [ ] If a migration slot needs renumbering (check `ls crates/argo-store/migrations/` for free slot), do so and avoid v60.rs.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `git push --force-with-lease origin claude-mission-ip-allowlist-deployment-mode`.
- [ ] PR #1463 mergeStateStatus returns to MERGEABLE.

## Branch + PR

- Branch: `claude-mission-ip-allowlist-deployment-mode` (existing).
- PR title: existing.

## Notes for the implementer

- ~10-30 min mechanical rebase depending on conflict surface.
- `--force-with-lease` only.
- Per-agent 2026-05-13T10:30Z triage: this PR is recovery-brief candidate (14h+ DIRTY with fix-recipe posted, no worker action).
