# claude-rebase-pr-2088-and-2092-second-recovery

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `0a600c51e` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-rebase-pr-2088-and-2092-second-recovery`.

🔴 **P0 PROCESS — #2088 + #2092 second-recovery rebase.** Source: scheduled audit `2026-05-23T09-40` §4 P0 #1 + #2 ("Rebase + drive #2088 to MERGED this tick" / "Rebase + drive #2092 to MERGED this tick"). Both PRs have been DIRTY/CONFLICTING since 2026-05-23T06:02Z (8h+ as of audit, carried across two audits). The earlier tick-85 cluster brief MERGED via #2102 but the actual #2088 + #2092 PR branches did NOT get their bits force-pushed cleanly — see ledger rows lines 209 + 213 in `docs/build-plan/BUILD_STATUS.md`.

**Why this is a single brief covering two PRs:** same shape, same recipe, both blocking mission-claim coverage (#2088 → claim-2 broken-wire; #2092 → claim-1/4 cross-tenant skills table). Manager-worker split is allowed (2 children) but not required — one worker can do both serially.

## Audit quotes verbatim

From `docs/skycot/audit/scheduled/2026-05-23T09-40.md` §4:

> **🔴 P0 (repeat from T03-39, sharpened)**: **Rebase + drive `#2088` to MERGED this tick.** Single remaining mission-claim broken-wire on main — claim-2 (Delegation is the unit) cannot flip LIVE until DutyScheduler invokes AgentRunner + emits audit row. CONFLICTING 8h+ across two audits. `eval-claim-2-live` brief BLOCKED solely on this.
>
> **🔴 P0 (repeat from T03-39, sharpened)**: **Rebase + drive `#2092` to MERGED this tick.** Skill table tenant_id sweep — 8 of 9 SkillStore methods still ungated. Cross-tenant data-table P0 for claim-1 + claim-4.

## Scope

For each of the 2 PRs (#2088, #2092):

1. **Pull the PR branch fresh from origin.**
   ```bash
   git fetch origin pull/2088/head:claude-fix-duty-scheduler-invokes-agent-runner-fresh
   git checkout claude-fix-duty-scheduler-invokes-agent-runner-fresh
   git rebase origin/main
   ```
   Same for `2092` / `claude-mission-skill-table-add-tenant-id-column`.

2. **Resolve conflicts.** Most likely conflict surfaces from the 21-PR landing wave since tick-85:
   - `docs/build-plan/BUILD_STATUS.md` — keep main's version (newer ledger state).
   - `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` — keep main's version.
   - `crates/argo-audit/src/sink_tx.rs` + `crates/argo-store/src/orchestrator/persist_handoffs.rs` — sibling work from #2090/#2100/#2103 may have moved nearby APIs; take "ours" for the PR's actual code, then re-test.

3. **Re-address the still-open fix-request items (per tick-91 ledger note on each row):**

   ### #2088 — fix-duty-scheduler-invokes-agent-runner
   Tick-91 fix-request requires:
   - (a) rebase onto current origin/main (HEAD `0a600c51e`);
   - (b) re-apply the cron_duty_e2e_test.rs split (was 512 LOC; split into sibling `cron_duty_wire_closure_test.rs` 316+280 LOC);
   - (c) re-apply the `DutyScheduler::tick` refactor to commit session + audit in one sqlx tx via `OrchestratorStore::insert_session_in_tx` + `AuditSink::write_in_tx_for_tenant`;
   - (d) re-apply the `duty_tick_rolls_back_session_when_audit_write_fails` test (asserts session row absent when audit fails — drop audit_record mid-tick);
   - (e) verify production hits with: `git grep -n "insert_session_in_tx\|WorkerDutyFired\|write_in_tx_for_tenant" -- 'crates/*.rs'` MUST show hits in production code, not just FINDINGS-ONLY ignored tests.

   ### #2092 — mission-skill-table-add-tenant-id-column
   Tick-91 fix-request requires:
   - (a) rebase onto current origin/main (HEAD `0a600c51e`);
   - (b) tenant-gate ALL 9 `SkillStore` methods (currently only `available_for_worker` is gated — 8 of 9 remain ungated per the audit);
   - (c) add cross-tenant negative tests beyond `available_for_worker` (every gated method gets a cross-tenant negative — same shape as the round-13 SQL-leak fixes shipped via #2094 + #2095);
   - (d) keep the additive-constructor `install_for_tenant*` pattern; do not break callers.

4. **Force-push-with-lease** to each PR's branch (NOT regular `--force`):
   ```bash
   git push --force-with-lease origin claude-fix-duty-scheduler-invokes-agent-runner
   git push --force-with-lease origin claude-mission-skill-table-add-tenant-id-column
   ```

5. **Don't admin-merge** — the conductor will Tier-1 review + merge each rebased PR. Worker's job ends at clean `gh pr view <N>` showing MERGEABLE/CLEAN.

### Conflict resolution philosophy

Per #2041 + #2102 pattern: keep main's version of gap-manifest / BUILD_STATUS rows (they reflect newer state). Take "ours" for the PR's actual code changes. Document any non-obvious resolution in the PR body force-push comment.

## Parallelism

- Manager-worker allowed: yes (2 PRs in parallel — one worker per PR is ideal)
- Max children: 2
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: `crates/argo-audit/src/sink_tx.rs`, `crates/argo-store/src/orchestrator/persist_handoffs.rs`, `crates/argo-skill/src/store.rs`, `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md`.

## Files you may touch

- Each PR's existing modified files (per its branch).
- Conflict-resolution files listed in §2 above.
- `docs/build-plan/BUILD_STATUS.md` — flip each row CHANGES-REQUESTED → IN_REVIEW after force-push.

## Files you must NOT touch

- The other PR's scope (no cross-PR commits in a single force-push).
- Unrelated production code.
- The original briefs at `streams/fix-duty-scheduler-invokes-agent-runner.md` and `streams/mission-skill-table-add-tenant-id-column.md` (they're correct; the fix is in the PR code).

## Acceptance criteria

- [ ] Both PRs rebased onto current origin/main `0a600c51e` (or whatever HEAD when claimed).
- [ ] **#2088**:
  - [ ] test file <500 LOC (split into sibling if needed),
  - [ ] atomic-tx via `sink_tx` + `insert_session_in_tx` + `write_in_tx_for_tenant`,
  - [ ] failure-injection asserts rollback,
  - [ ] `git grep "WorkerDutyFired"` shows production hits.
- [ ] **#2092**:
  - [ ] all 9 `SkillStore` methods bind tenant_id at SQL,
  - [ ] cross-tenant negative tests cover every gated method,
  - [ ] additive-constructor pattern preserved.
- [ ] Both force-pushed with-lease (NOT regular force-push).
- [ ] Each PR's `gh pr view <N>` reports `mergeable: MERGEABLE`, `mergeStateStatus: CLEAN`.
- [ ] `cargo clippy -p argo-orchestrator -p argo-skill --all-targets -- -D warnings` clean.
- [ ] PR-body force-push summary comment cites this brief + the audit `2026-05-23T09-40` §4 P0 #1 + #2.

## Branch + PR

- Worker pushes directly to existing PR branches `claude-fix-duty-scheduler-invokes-agent-runner` + `claude-mission-skill-table-add-tenant-id-column`.
- **No new PR** is opened by this brief — it's a recovery commit on existing PRs.
- This brief's own working branch can be `claude-rebase-pr-2088-and-2092-second-recovery` for ledger bookkeeping only; the ledger row flips when both target PRs are MERGED.

## Notes for the implementer

- ~300-600 LOC across the 2 PRs combined.
- **Reference #2102 (tick-85 cluster) as the playbook.** Same shape (per-PR rebase + cherry-pick fixes + push-with-lease + conductor-coordinated-merge). #2102's only failure was that #2088 + #2092 didn't end up with all the bits on their actual PR branches — this brief fixes that.
- **The 2 PRs are independent surfaces** — no cross-PR conflicts expected; serial or parallel.
- **Cite audit `2026-05-23T09-40` §4 P0 #1 + #2 verbatim** in each force-push PR-body comment.
- After this brief MERGES both PRs, the `eval-claim-2-live-duty-cron-fires-runner-test` row (currently BLOCKED at ledger line 143) should be flipped to TODO so a worker can claim it — that's the regression test pinning claim-2 LIVE.
