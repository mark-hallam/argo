# codex-policy-as-code-enforcement — Resolve real tenant/role/roster + enforce quiet_hours/signatures/audit_chain (F-06 P1)

**Owner**: codex. **Depends on**: `codex-toolcontext-tenant` already MERGED #1727 (supplies tenant).
**Authored against**: origin/main `58e853b6` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-policy-as-code-enforcement`.
**Source**: F-06 P1 in `docs/skycot/audit/2026-05-16-one-shot-architecture-audit.md`.

## Mission alignment

- **Claim 1 (teammate):** Each worker has an `auto_approval_policy` (`docs/skycot/MISSION.md:56`).
- **Claim 2 (delegation):** Delegation with policies and approval routing.
- **Claim 6 (multi-channel approvals):** Approval gates respect risk + budget + time.
- **V1 success scenario #4:** Auto-approval under budget WITH weekday business hours (`docs/skycot/MISSION.md:223`) — currently NOT enforced; quiet_hours field is inert.

## Scope

P1 correctness — enterprise policy story is paper. `argo-identity::policy` parses a rich resolution model (`crates/argo-identity/src/policy.rs:17`):
- Action-specific rules
- Worker policy
- Roster policy (`MISSION_BACKLOG.md:172` flags roster lookup as open)
- Workspace policy
- Workspace defaults

It has role + tenant selectors (`policy.rs:83`) and effective fields for `require_signatures`, `quiet_hours`, `audit_chain_required` (`policy.rs:187`).

**But** `worker_actions_auto_approval` calls `resolve_effective_policy` with empty role + tenant strings (`worker_actions_auto_approval.rs:192`, `:195`, `:196`). The comment at `:179`, `:183` acknowledges "role and tenant context are not threaded through." The gate then consumes only `effective.auto_approval` (`:199`) — ignoring signatures/quiet_hours/audit_chain_required entirely.

Result: non-wildcard tenant/role policies are inert; roster policy inheritance doesn't happen; quiet hours + signatures + audit-chain-required flags are paper. V1 success scenario #4 (auto-approve weekdays business hours) currently passes any time of day.

Scope:
- Pass real `tenant_id`, `role`, `worker`, and `roster` to `resolve_effective_policy`. Tenant comes from `ToolContext.tenant_id` (now wired post-F-004); role/worker from session context; roster from `argo-identity::roster::Roster` lookup.
- After resolution, enforce ALL effective fields in the gate, not just `auto_approval`:
  - `require_signatures` (N-of-M): if non-zero and current approval doesn't meet threshold, FAIL CLOSED — escalate to manual.
  - `quiet_hours`: parse the time window, compare to current wall-clock + tenant timezone, gate accordingly.
  - `audit_chain_required`: assert AuditSink is wired and chain-verified for the session; otherwise fail closed.
- Add 5 test cases covering: non-wildcard tenant policy, role-specific policy, roster inheritance, quiet hours window edge, N-of-M signature shortfall.

LOC: ~400.

## Files you may touch

- `crates/argo-cli/src/handlers/worker_actions_auto_approval.rs` — replace empty-string role/tenant calls; consume + enforce all effective fields. ≤150 modified lines.
- `crates/argo-identity/src/policy.rs` — narrowly: add helper for time-of-day quiet-hours check if not present; expose `audit_chain_required` to gate. ≤60 modified lines.
- `crates/argo-identity/src/roster.rs` (or `roster_store.rs`) — narrowly: ensure `resolve_for_worker(tenant, worker_id) -> Option<RosterId>` is callable from the gate. ≤40 modified lines.
- `crates/argo-cli/src/handlers/worker_actions_auto_approval_tests.rs` (NEW or sibling extend) — 5 test cases. ≤180 LOC.
- `crates/argo-cli/src/handlers/session_context.rs` (if exists) or wherever session context is constructed — narrowly: ensure role + worker_id are addressable from the gate site. ≤30 modified lines.

## Files you must NOT touch

- `crates/argo-audit/**` — audit-chain is correct; just consume it.
- Channel sender code — that's F-05/F-07 scope.

## Acceptance criteria

- [ ] `resolve_effective_policy` is called with non-empty tenant + role + worker + (optional) roster.
- [ ] `require_signatures > 0` enforced: insufficient signatures → escalate (no silent auto-approve).
- [ ] `quiet_hours` parsed + enforced against tenant timezone.
- [ ] `audit_chain_required = true` enforced: gate fails closed if AuditSink isn't chain-verified.
- [ ] 5 regression tests pass (tenant-specific, role-specific, roster-inheritance, quiet-hours-edge, N-of-M shortfall).
- [ ] `cargo test -p argo-cli -p argo-identity worker_actions_auto_approval` passes.
- [ ] `cargo clippy -p argo-cli -p argo-identity --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-policy-as-code-enforcement`
- PR title: `feat(approval): enforce policy-as-code tenant/role/roster + quiet hours + signatures (codex-policy-as-code-enforcement) — F-06 P1`

## Notes for the implementer

- "Fail closed" is the default for any unsupported effective field. Don't silently no-op.
- Quiet hours timezone: derive from tenant config (`workspace.timezone` or similar). If absent, use UTC + log a warning that quiet hours are inexact.
- Audit-chain requirement: check that `AuditSink::new_with_tenant` is in use (not the deprecated bare `new`) — post-F-005b this is checkable structurally.
- After this brief lands, V1 success scenario #4 actually enforces the named constraints.
