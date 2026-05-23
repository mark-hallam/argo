# Round 30 — Claim 4 MVP row 6 — gateway promote audit-emit pin gap

- **Date**: 2026-05-23
- **Round**: 30
- **Pillar/Claim**: Claim 4 (Skills shared + learned) — MVP row 6
  (operator-approve flow + `skill.candidate.promoted` audit emit)
- **Verdict**: 🟡 **PARTIAL** (substrate green; gateway-side regression weak)
  — **P2 brief candidate**

## Setup

Round 29 closed Claim 4 row 4 (novel-sequence detection); rows 5 + 6
remain 🟢-substrate-only per round-29's note. I traced row 6 by:

1. Reading both production emitters of `AuditKind::SkillCandidatePromoted`:
   - `crates/argo-cli/src/handlers/skill_review_candidates.rs:163`
     (memory-store-side `promote_candidate` helper, CLI-driven)
   - `crates/argo-cli/src/handlers/gateway/routes_worker_skills.rs:326`
     (gateway POST `/api/v1/workers/<id>/skills/<sid>/promote`)
2. Reading the existing tests pinning each surface:
   - `crates/argo-cli/src/handlers/skill_review_candidates_tests.rs`
     (7 tests including payload-shape pins for memory-side)
   - `crates/argo-cli/src/handlers/gateway/routes_worker_skills_tests.rs`
     (13 tests including 2 promote tests at lines 366-434)
3. Running all 13 gateway tests cold.

## Observed

### Memory-side promote (CLI) is well-pinned

`skill_review_candidates_tests.rs:91-119`
(`promote_attributes_audit_row_to_originating_session`) and lines
121-154 (`promote_records_feedback_in_audit_payload_when_present`)
already pin the full audit-row contract on the CLI memory-store path:

- `kind == "skill.candidate.promoted"` (matched by string label)
- `payload[vertical_id] == "gtm-engineer"`
- `payload[skill_id]` is a string
- `payload[memory_entry_id] == entry.id.to_string()`
- `payload[feedback][action_id] == "action-123"` when provided
- `payload[feedback][vote] == "up"` when provided
- Audit row is attributed to the originating session via
  `rows_for_session` (not just any session)

These tests would flip red if the CLI promote helper stopped emitting
the audit row, mis-stamped the session, or dropped any of those
payload fields. **Strong anti-theater shape.**

### Gateway-side promote (HTTP) is weakly pinned

`routes_worker_skills_tests.rs:366-403`
(`promote_attaches_skill_and_writes_audit_row`):

```rust
let count: i64 = sqlx::query_scalar(
    "SELECT COUNT(*) FROM audit_record WHERE kind = 'skill.candidate.promoted'",
)
.fetch_one(store.pool())
.await
.expect("count audit");
assert!(count >= 1, "promote should write audit row, found {count}");
```

That's the *only* audit-row assertion. The SELECT does not filter by
tenant_id, does not inspect payload at all, does not check the
denormalized `tenant_id` column, does not pin `worker_id` or
`promotion_kind` payload fields. **A refactor that:**

- Drops `payload[skill_id]`, replaces it with garbage UUID, or
- Inverts `promotion_kind` value (e.g. `"learned_to_archived"`), or
- Drops the `AuditSink::new_with_tenant(...)` call in
  `routes_worker_skills_helpers.rs:169` (so the denorm tenant_id
  column is NULL/system instead of ctx.tenant), or
- Allows tenant A's promote to leak a row visible to a tenant B
  filter

— would **not flip this test red**. The test passes purely on row
existence.

### Production code is sound

`routes_worker_skills_helpers.rs:151-175` (the gateway's `emit_audit`
helper) does the right thing: builds `payload[tenant_id, worker_id,
endpoint]` from `ctx`, splats `extra` over it, and writes via
`AuditSink::new_with_tenant(store, ctx.tenant.to_string())` so the
denorm column is stamped. The path is correct — there's just no
falsifiable regression guarding it.

## Anti-theater check

I ran `cargo test -p argo-cli --bin argo
handlers::gateway::routes_worker_skills 2>&1 | tail` — 13/13 pass in
0.57s. The 13-test suite covers cross-tenant 404 on GET + DELETE, but
NOT cross-tenant on promote. I traced through `promote_response` line
296-343 — when a foreign worker_id is passed,
`helpers::resolve_worker_ctx` returns a 404, so cross-tenant promote
*does* fail at the routing layer. The gap is at the AUDIT-WRITE level:
no test pins that a tenant A promote leaves zero visible rows in a
tenant B audit-feed filter.

The skill-loop closure smoke at
`crates/argo-cli/tests/skill_loop_closure_smoke.rs:129-142` emits the
audit row **inside the test** with the comment "The CU dispatch shim
does not yet emit a candidate-proposed audit row" — it's a manual
emit, not a verification of any production emit path. So that test
doesn't fill the gap either.

## Gap / follow-up brief candidate

**Brief name**: `mission-claim-4-mvp-row-6-gateway-promote-audit-pin`

**Priority**: P2 (substrate is correct, this is a regression-guard
hardening; not user-visible)

**Scope** (~120-180 LOC, single new test file):

`crates/argo-cli/tests/claim_4_mvp_row_6_gateway_promote_audit.rs` —
4 tokio tests pinning the gateway promote audit-emit contract:

1. **payload shape contract**: POST
   `/api/v1/workers/<id>/skills/<sid>/promote` writes an audit row
   where `kind == "skill.candidate.promoted"` AND `payload[endpoint]
   == "worker.skills.promoted"` AND `payload[skill_id] ==
   skill_id.to_string()` AND `payload[promotion_kind] ==
   "learned_to_active"` AND `payload[worker_id] ==
   worker_id.to_string()` AND `payload[tenant_id] ==
   ctx.tenant.to_string()`. Six assertions on the same row.
2. **denorm tenant_id column contract**: The `audit_record.tenant_id`
   column (not just the payload) matches `ctx.tenant.to_string()`.
   Pins the `AuditSink::new_with_tenant(...)` call site so it can't
   silently regress to plain `AuditSink::new(...)` (which would write
   `SYSTEM_AUDIT_TENANT_ID` to the denorm column).
3. **cross-tenant isolation contract**: Setup two workers in two
   tenants. Tenant A POSTs promote. A SELECT `WHERE tenant_id =
   tenant_b` returns ZERO `skill.candidate.promoted` rows. Pins that
   the audit row is tenant-bound and does NOT leak.
4. **trust ratchet + audit emit are atomic**: After ONE promote
   POST, BOTH `worker_skill.proficiency == 5` AND ONE audit row are
   present. Pins the two side-effects-in-one-handler invariant so a
   refactor that splits them and drops one half flips red.

**Why this matters**: Claim 4 row 6 is the operator-approve flow.
The audit trail is what makes "operator decided X" defensible after
the fact (`/audit` page, hash-chain verification, compliance). If a
refactor breaks the payload shape, the SPA `/audit` filter that
groups promote events by worker_id breaks silently; if the denorm
column is wrong, the tenant filter on the audit feed breaks
silently; if cross-tenant leaks, tenant B sees tenant A's
proprietary skill names. None of these have a current regression
guard.

**Why I'm not authoring it this round**: The hourly-scheduled-task
evaluator charter is read-only across code; my three write paths are
`docs/skycot/evals/scheduled/**`, `docs/skycot/agents/heartbeats/evaluator.json`,
and `docs/skycot/agents/SHARED.md`. The conductor (Step 8) decides
whether to author the brief. If the operator wants the
interactive-evaluator pattern of authoring tests directly, the
charter needs an explicit amendment for the scheduled-task variant.

## Operational

- Claim 4 MVP score holds at ~87% after this round (no change —
  this is a finding, not a test add). Authoring the recommended
  brief + landing it would push to ~94%.
- Claim 4 MVP row 5 (pending_review review-queue operator-facing
  surface) is the only other 🟢-substrate row remaining. Round 29
  already pins the SkillStore-side lifecycle transition; the gap is
  the CLI/gateway surface (`argo skill review` + GET
  `/skills/<name>/pending`). That's a separate brief shape, parked
  for a future round.
