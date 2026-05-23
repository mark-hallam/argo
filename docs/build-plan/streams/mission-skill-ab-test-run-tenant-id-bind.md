# mission-skill-ab-test-run-tenant-id-bind

**Owner**: claude.
**Depends on**: none (this is the runtime side; storage side `mission-skill-table-add-tenant-id-column` ships independently).
**Authored against**: origin/main `<latest>` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-skill-ab-test-run-tenant-id-bind`.

🔴 **P0 CROSS-TENANT SKILL A/B-RUN CORRUPTION.** Source: evaluator round-9 PR #2081 finding §2 P0 #2.1.

> "`skill_ab_test_run` SELECT at `turn_skills_runtime.rs:159-165` binds only `skill_name`, no `tenant_id` → cross-tenant A/B run corruption."

The skill A/B-test runtime selects rows by skill_name alone. If tenant_a and tenant_b both have a skill named `slack-dm` (legitimate — same canonical name from different sources), the runtime mixes their A/B-test variants, corrupting both tenants' metrics + potentially routing tenant_a's worker to tenant_b's variant.

## Scope

Bind `tenant_id` to the `skill_ab_test_run` query + every other cross-tenant-blind query in `turn_skills_runtime.rs`.

### Implementation

`crates/argo-agent-runner/src/turn_skills_runtime.rs:159-165` — find the SELECT, add `AND tenant_id = ?` predicate, thread tenant_id from the turn context (worker.tenant_id).

If the `skill_ab_test_run` table itself lacks a tenant_id column, file a parallel brief (`mission-skill-ab-test-run-table-add-tenant-id`) and BLOCK this brief on it. If the column exists but isn't bound, this brief is a one-callsite fix.

### Cross-tenant negative test

Sibling test: bootstrap tenant_a + tenant_b each with a skill named `test-skill` + an A/B-test run for each. Call from tenant_b's turn context → assert returns tenant_b's variant, NOT tenant_a's. Try with a foreign skill_name → assert returns Err(NotFound), NOT a leaked variant.

### Audit chain

Per memory `feedback_enterprise_grade_defaults.md`: every cross-tenant probe gets an audit row. Add `SkillAbTestRunQueried { tenant_id, skill_name }` to the runtime so cross-tenant attempts are observable.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-agent-runner/src/turn_skills_runtime.rs` — narrowly scoped: fix the SELECT.
- `crates/argo-agent-runner/src/turn_skills_runtime_tests.rs` (or new sibling) — cross-tenant negative.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- The skill IR / promotion / scoring logic.
- The skill table schema (separate brief if it needs migration).
- Other turn_*.rs files.

## Acceptance criteria

- [ ] `skill_ab_test_run` query at line 159-165 binds tenant_id.
- [ ] Other cross-tenant-blind queries in same file swept too (audit for more before shipping).
- [ ] Cross-tenant negative test: tenant_b's call returns ONLY tenant_b's variant.
- [ ] If the table lacks tenant_id column, BLOCK + file dep-brief.
- [ ] `cargo clippy -p argo-agent-runner --all-targets -- -D warnings` clean.
- [ ] Every modified file <500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `mission-skill-ab-test-run-tenant-id-bind`
- PR title: `fix(agent-runner): bind tenant_id to skill_ab_test_run SELECT (mission-skill-ab-test-run-tenant-id-bind)`

## Notes for the implementer

- ~80-180 LOC expected. SQL fix ~10, cross-tenant test ~80, sweep of sibling queries ~30.
- **Pre-flight check**: confirm `skill_ab_test_run` table HAS `tenant_id` column. If not, this brief depends on the storage-side migration (`mission-skill-table-add-tenant-id-column` for the parent `skill` table, OR a new sibling migration).
- **Don't refactor `turn_skills_runtime.rs`** — this is a pin-point fix.
- Cite round-9 evaluator + #2081 §2.1 verbatim.
