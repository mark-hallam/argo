# claude-rebase-pr-1852 — recover #1852 mission-pm-vertical-workflow-skill-invoke

**Owner**: claude. **Recovery target**: PR [#1852](https://github.com/mark-hallam/Argo-Agent/pull/1852). **Original brief**: `streams/mission-pm-vertical-workflow-skill-invoke.md`.
**Authored against**: origin/main `58d3e8a1` on `2026-05-20`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-rebase-pr-1852`.
**Priority**: P1 — finishes mission claim 4's vertical workflow skill_invoke step (Phase 5 was scaffolded then paused).

## Why this brief exists

PR #1852 was opened 2026-05-19T12:22Z by a claude worker. Substantively complete (+634/-26 across 8 files: argo-audit event kinds, argo-cli vertical_workflow handlers + tests). Worker did not push again after the initial open; PR has been DIRTY against `origin/main` since at least 2026-05-19T13:00Z (many subsequent merges have moved main forward — #1790 fn-LOC splits, #1846 workflow_definition, #1847 mission-claim-3 routing, #1856 trust-streak, #1832 signup-protection, #1850-#1861 kill-switch family, #1860 workflow-edit-api, several audit-track merges).

No fix-request comments are outstanding on the PR — it's just stale. The 30+ hours of upstream traffic introduces real conflicts; a fresh rebase is the path forward.

## Acceptance: pick ONE recovery path

### Path A — rebase (preferred — preserves existing work)

1. `git fetch origin && git checkout claude-mission-pm-vertical-workflow-skill-invoke`
2. `git rebase origin/main`. Expected conflicts:
   - **`crates/argo-audit/src/event.rs`** — `AuditKind` enum grew during the wait (added: `WorkerFreezeBlocked` from #1850, `RoleBindingChanged`/`DynamicCapabilityGranted` from #1842, `WorkflowDefinitionCreated`/`WorkflowHeadChanged`/`WorkflowVersionArchived` from #1860, `SkillRegressionDetected` from #1851, possibly `RoutingDecision` rendering changes from #1847). Resolve by preserving ALL incoming variants alongside this PR's `SkillInvokeStarted`/`SkillInvokeCompleted` (or whatever the PR adds) — accretion-only file.
   - **`crates/argo-cli/src/handlers/mod.rs`** — module registration; multiple briefs have added `pub mod xxx;` lines. Accept union of all.
   - **Possibly** `crates/argo-cli/src/handlers/vertical_workflow.rs` if anything else touched it; unlikely but check.
3. Verification per original brief: `cargo clippy -p argo-cli -p argo-audit --all-targets -- -D warnings` clean; `cargo test -p argo-cli vertical_workflow` passes; `bash scripts/workspace-clippy-fast-fail.sh` clean.
4. `git push --force-with-lease origin claude-mission-pm-vertical-workflow-skill-invoke`.
5. Comment on #1852: "✅ rebased + force-pushed onto origin/main `<sha>`. Conflicts resolved in argo-audit/event.rs (AuditKind enum accretion) and argo-cli/handlers/mod.rs (module registration union)."

### Path B — fresh start

Only if Path A's rebase exceeds ~5 files of conflict. Open `mission-pm-vertical-workflow-skill-invoke-v2` PR against current main re-applying the same intent.

## Acceptance criteria

- [ ] PR #1852 ends MERGEABLE / non-DIRTY against current main.
- [ ] All PR-original tests pass: `cargo test -p argo-cli vertical_workflow`.
- [ ] `cargo clippy -p argo-cli -p argo-audit --all-targets -- -D warnings` clean.
- [ ] `bash scripts/workspace-clippy-fast-fail.sh` clean.
- [ ] No regression in existing argo-audit `AuditKind::all()` count tests.
- [ ] Every modified file < 500 LOC (no LOC waiver for this brief — the work is small enough to stay under).

## Notes for the implementer

- This recovery brief follows the proven 5-time pattern this session (1790, 1799, 1832, 1845, 1858). Worker should expect ~10-20 min of rebase + verification work.
- If `cargo test -p argo-cli vertical_workflow` exposes a test the PR depended on that has since changed shape (e.g. a test fixture for `WorkflowDefinition` from #1846), update the test accordingly — those are mechanical follow-ups.
- The `AuditKind` enum is the known accretion hotspot — every brief that adds a variant changes this file. Worker should accept ALL incoming + add this PR's.
