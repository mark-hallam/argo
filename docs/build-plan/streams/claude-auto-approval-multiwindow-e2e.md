# claude-auto-approval-multiwindow-e2e — Auto-approval policy + multi-channel fallback E2E (scenario #4 polish)

**Owner**: claude. **Depends on**: M8-orch-multi-channel-approval-composition MERGED ✅ (now satisfied via #1661).
**Authored against**: origin/main `03573bda` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-auto-approval-multiwindow-e2e`.

## Scope

Phase C audit P1.5 follow-up (`docs/skycot/audit/2026-05-14-phase-C-mission-claim-coverage.md` §3 #4). The existing `auto_approval_budget_e2e.rs` covers single-day budget enforcement. This brief extends it to cover:

1. **Multi-day windows**: a policy that allows $5/action up to $50/day; verify both ceilings fire.
2. **Weekday-only business hours**: a policy that allows auto-approve only Mon-Fri 9am-5pm; verify off-hours actions go to manual approval.
3. **Channel fallback routing**: when a budget-exceeding action is encountered, the worker routes the approval request via M8 composition (e.g., Slack → SMS fallback). Test that fallback fires correctly.

Together this demonstrates **MISSION.md success scenario #4** ("Auto-approval under a budget works") fully.

LOC: ~180.

## Files you may touch

- NEW `crates/argo-cli/tests/auto_approval_multiwindow_e2e.rs` — integration test extending the existing budget E2E. Uses fixture clocks for the time-window check.
- `scripts/smoke-skycot.sh` — narrowly: add as a smoke step (conditional; skip if e2e infra not available). ≤10 lines.

## Files you must NOT touch

- The auto-approval policy production code in argo-identity (consume the existing API).
- M8 composition code in argo-orchestrator (consume).
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] `cargo test --test auto_approval_multiwindow_e2e -p argo-cli` passes.
- [ ] 3 sub-tests: per-action ceiling, per-day ceiling, off-hours route-to-manual.
- [ ] 1 sub-test: budget-exceeding action triggers M8 channel-fallback (Slack mock → SMS mock).
- [ ] Test is deterministic (fixture clocks; no real time-based behavior).
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `claude-auto-approval-multiwindow-e2e`
- PR title: `test(approval): multi-window auto-approval + channel fallback E2E (claude-auto-approval-multiwindow-e2e)`

## Notes for the implementer

- Mission-claim alignment: scenario #4 (auto-approval under budget) + claim 6 (multi-channel approval).
- M8 just MERGED (#1661); the channel-fallback composition is now available to call from tests.
- Drift-prevention: adds NEW test file only; no new pub types.
