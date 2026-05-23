# mission-skill-refine-add-rbac-audit-and-test-coverage

**Authored against:** `origin/main` 14f44211d on 2026-05-24.
**Validate before claim:** `bash scripts/check-brief-freshness.sh mission-skill-refine-add-rbac-audit-and-test-coverage`.

## Why this brief exists

Surfaced as F-G1 from PR #2120 (`eval-argo-cli-gateway-module-sweep`, MERGED 2026-05-23). The `routes_skill_refine` handler in `crates/argo-cli/src/handlers/gateway/routes_skill_refine.rs` is missing three things every other gateway handler in `routes_worker_skills.rs` / `routes_skills.rs` / `routes_workflow_execute.rs` carries:

1. **RBAC capability check** — the audit found no `rbac::require_capability(...)` call gating the refine endpoint, meaning any authenticated principal can refine any skill in their tenant (or worse, across tenants if the path doesn't tenant-bind).
2. **Audit-row emission** — refining a skill is a mutating operation but no `skill.refined` (or equivalent) audit row is currently emitted, so the audit-chain has no record of who refined what.
3. **Cross-tenant negative test** — there's no test pinning that tenant_b can't refine tenant_a's skills.

PR #2120 landed 2 `#[ignore]`'d FINDINGS-ONLY tests pinning the post-fix behavior; this brief un-ignores them.

## Acceptance criteria

1. Add `rbac::require_capability(...)` (the relevant capability is likely `Skill::Refine` or `Skill::Edit` — claim-time worker confirms by reading sibling handlers).
2. Emit a `skill.refined` audit row (or equivalent — check the AuditKind enum) on successful refine, carrying `payload[skill_id, name, vertical_id, refined_by, tenant_id]` + denorm `tenant_id` column.
3. Un-ignore the 2 FINDINGS-ONLY tests in the test file PR #2120 left behind (claim-time worker greps for `#[ignore]` in `crates/argo-cli/tests/` or the dedicated test file PR #2120 created — find via `grep -r "routes_skill_refine" crates/argo-cli/tests/`).
4. Add a 3rd test: cross-tenant negative — tenant_b POSTs to refine tenant_a's skill, gets `403 Forbidden` (or `404` if existence-leak protection is required), no audit row written to either tenant's chain.
5. `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
6. `cargo test -p argo-cli routes_skill_refine` green.

## Parallel-surface declarations

- `routes_skill_refine` already exists — this is a hardening brief, not a new-surface brief.
- `AuditKind` may or may not already carry the `SkillRefined` variant; check `crates/argo-audit/src/event_kind.rs` — if missing, add it per the same pattern as `SkillApproved` / `SkillRejected` / `SkillCandidatePromoted`.
- No new pub structs / enums introduced.

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_skill_refine.rs` — primary edit.
- `crates/argo-audit/src/event_kind.rs` + `event.rs` + `event_tests.rs` — if `SkillRefined` variant doesn't exist (variant-count bump per HARD RULE 5 exhaustive-match allowance).
- `crates/argo-cli/tests/<existing-skill-refine-tests-file>.rs` — un-ignore FINDINGS-ONLY + add cross-tenant negative.

## Boundary

- Do NOT touch other gateway handlers — F-G1 is the single named finding; the other 3 🟡 nits on worker-settings are separate.
- Do NOT add new helpers in `routes_worker_skills_helpers.rs` unless reused — keep the change surgical.
- Pre-existing `argo-skill` workspace-clippy lints (`claim_4_mvp_row_5_pending_review_queue.rs` + `claim_4_mvp_row_6_operator_approve_emits_audit.rs`) are not in this brief's may-touch (separate brief: `fix-skill-test-claim-4-row-5-6-workspace-clippy-lints`).

## Sizing

~150-300 LOC (production + tests). Includes possible AuditKind variant add (~30 LOC) if `SkillRefined` doesn't already exist.

**🟡 P1 RBAC + AUDIT HARDENING** — closes F-G1 from `eval-argo-cli-gateway-module-sweep` audit (PR #2120). Same shape as the cross-tenant approval-leak that landed in `mission-approval-decide-request-tenant-pinned` (#2146): SQL might be right, but no test pins it.
