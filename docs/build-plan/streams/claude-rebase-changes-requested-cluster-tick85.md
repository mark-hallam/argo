# claude-rebase-changes-requested-cluster-tick85

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `<latest>` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-rebase-changes-requested-cluster-tick85`.

🔴 **P0 PROCESS — tick-85 CHANGES-REQUESTED cluster stale 13h+, 2 of 3 now CONFLICTING.** Source: evaluator round-17 SHARED.md handoff 2026-05-23T15:00:00Z.

> "🔴 NEW P0 PROCESS FINDING: tick-85 CHANGES-REQUESTED cluster (#2087 cred-get 91/91 callsites + #2088 duty-scheduler + #2090 per-tenant master-key) stale 13h+ with no worker action; #2088+#2090 now CONFLICTING. RECOMMEND BRIEF: claude-rebase-changes-requested-cluster-tick85 (P0, same recipe as #2041 5-DIRTY rebase cluster) — demo MP4 cannot ship cleanly while these 3 P0 cross-tenant fixes rot."

Three load-bearing P0 fixes are rotting. Same operational pattern as the tick-38 5-DIRTY cluster cleared by #2041. Sweep coordinated.

## Scope

For each of the 3 PRs (#2087, #2088, #2090):

1. **Pull the PR branch**, rebase onto current `origin/main`, resolve conflicts cleanly (most likely gap-manifest / BUILD_STATUS / sibling-test files).
2. **Address the specific fix-request comments** posted by conductor in tick-85:
   - **#2087 mission-cred-get-tenant-parameter**: thread tenant_id into ~15 channel/connector constructors that have tenant context. Sites that genuinely need `get_unscoped` get a `// tenant-unscoped: <real reason>` comment. The brief allowed manager-worker split (4 children); use that if appropriate.
   - **#2088 fix-duty-scheduler-invokes-agent-runner**: (a) split `cron_duty_e2e_test.rs` (currently 512 LOC) into a sibling file to satisfy the 500-LOC ceiling; (b) refactor `DutyScheduler::tick` to use `argo-audit/src/sink_tx.rs::run_transactional_audit` (the pattern from #2078) so audit + session writes share a transaction; (c) update the failure-injection test to assert atomic rollback (worker_duty.session row absent when audit fails) rather than forward-progress.
   - **#2090 mission-credentials-per-tenant-master-key**: wire `decrypt_value_with_fallback` into the production callsite at `crates/argo-config/src/cred.rs:92` (replace the bare `decrypt_value()` call). On read, if key_version=1 detected, re-encrypt with the per-tenant key + write back to the TOML section + emit a one-shot tracing::warn! per (tenant, key).
3. **Force-push with-lease** to each PR's branch.
4. **Don't admin-merge** — the conductor will Tier-1 review each rebased PR + merge.

### Conflict resolution philosophy

Per #2041 pattern: keep main's version of gap-manifest / BUILD_STATUS rows (they reflect newer state). Take "ours" for the PR's actual code changes. Document any non-obvious resolution in the PR body.

## Parallelism

- Manager-worker allowed: yes (3 PRs in parallel — one worker per PR is ideal)
- Max children: 3
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: [`crates/argo-config/src/cred.rs`, `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md`]

## Files you may touch

- Each PR's existing modified files (per its branch).
- `docs/build-plan/BUILD_STATUS.md` — flip each row CHANGES-REQUESTED → IN_REVIEW after force-push.

## Files you must NOT touch

- Other PRs' scope.
- Unrelated production code.
- The original briefs (they're correct; the fix is in the PR code).

## Acceptance criteria

- [ ] All 3 PRs rebased onto current origin/main.
- [ ] **#2087**: ~15 channel/connector callsites thread tenant_id; remaining `get_unscoped` sites have justified comments. PR body lists every site touched.
- [ ] **#2088**: test file <500 LOC, atomic-tx via sink_tx, failure-injection asserts rollback.
- [ ] **#2090**: `cred.rs:92` calls `decrypt_value_with_fallback` + write-back-on-Legacy + one-shot warn.
- [ ] All 3 force-pushed with-lease (NOT regular force-push).
- [ ] Each PR's `gh pr view` reports MERGEABLE/CLEAN.
- [ ] `cargo clippy -p <affected-crate> --all-targets -- -D warnings` clean for each PR.
- [ ] BUILD_STATUS.md updated with PR statuses.

## Branch + PR

- Branch: `claude-rebase-changes-requested-cluster-tick85`
- PR title: `chore: rebase + address fix-requests on tick-85 cluster (#2087/#2088/#2090) (claude-rebase-changes-requested-cluster-tick85)`

## Notes for the implementer

- ~600-1500 LOC across the 3 PRs combined. Manager-worker eligible.
- **Reference #2041 as the playbook** — same shape (cluster rebase + cherry-pick fixes + push-with-lease + conductor-coordinated-merge).
- **The 3 PRs are independent surfaces** — no cross-PR conflicts expected.
- **Each PR's worker should cite the round-17 evaluator handoff + the tick-85 fix-request comment verbatim in its updated PR body.**
- Cite round-17 evaluator handoff verbatim in this brief's PR body.
