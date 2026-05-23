# Round 37 · PR #2144 — `mission-claim-4-mvp-row-6-gateway-promote-audit-pin` anti-theater verify

- **Date**: 2026-05-24
- **Round**: 37 (scheduled-task hourly fire, read-only)
- **Pillar / Claim**: Claim 4 — Skills are shared and learned (gateway promote audit-emit)
- **PR**: [#2144](https://github.com/mark-hallam/argo/pull/2144) (MERGED, commit `a7d9d8884`)
- **Verdict**: 🟢🟢 **PASS** — GOLD-STANDARD ANTI-THEATER
- **Severity if it had failed**: P2 (audit-emit refactor could silently drop fields without test flipping red)

## Setup

```bash
git show --stat a7d9d8884
grep -n 'fn ' crates/argo-cli/src/handlers/gateway/claim_4_mvp_row_6_gateway_promote_audit_pin.rs
grep -n 'helpers::emit_audit\|new_with_tenant\|skill.candidate.promoted\|learned_to_active' \
  crates/argo-cli/src/handlers/gateway/ -r
```

## What the PR changes

Round-30's finding was that `routes_worker_skills_tests.rs:396-403`
asserted only:

```rust
let count: i64 = sqlx::query_scalar(
    "SELECT COUNT(*) FROM audit_record WHERE kind = 'skill.candidate.promoted'",
)...
assert!(count >= 1, "promote should write audit row, found {count}");
```

That is theater-grade. The following refactors would NOT flip it red:
- Drop `skill_id` from the audit payload
- Invert `promotion_kind: "learned_to_active"` to `"active_to_learned"`
- Replace `AuditSink::new_with_tenant(store, ctx.tenant)` with
  `AuditSink::new(store)` (falls back to `__legacy__`/`ws-default`
  sentinel for the denorm column)
- Leak rows into tenant_b's audit feed
- Drop `principal_id_for_audit(req)` extraction → all rows attribute
  to `system`

The PR adds 4 byte-exact pin tests under
`crates/argo-cli/src/handlers/gateway/claim_4_mvp_row_6_gateway_promote_audit_pin.rs`
+ a 3-line `#[cfg(test)] #[path = ...] mod` declaration in
`routes_worker_skills.rs` (because `promote_response()` is
`pub(super)` and external integration tests under `tests/` can't reach it).

## Observed (anti-theater proof)

### Production seam — read first, then compare to tests

Production audit-emit lives in two files, traced end-to-end:

`routes_worker_skills.rs:322-333`:
```rust
helpers::emit_audit(
    store,
    req,
    &ctx,
    AuditKind::SkillCandidatePromoted,
    "worker.skills.promoted",
    json!({
        "skill_id": skill_id.0.to_string(),
        "promotion_kind": "learned_to_active",
    }),
)
.await;
```

`routes_worker_skills_helpers.rs:151-175` (the actual sink call):
```rust
pub(super) async fn emit_audit(
    store: &Arc<SessionStore>,
    req: &str,
    ctx: &WorkerCtx,
    kind: AuditKind,
    endpoint: &str,
    extra: Value,
) {
    let mut payload = json!({
        "endpoint": endpoint,
        "tenant_id": ctx.tenant.to_string(),
        "worker_id": ctx.worker.id.to_string(),
    });
    if let (Some(map), Some(obj)) = (payload.as_object_mut(), extra.as_object()) {
        for (k, v) in obj { map.insert(k.clone(), v.clone()); }
    }
    if let Err(e) = AuditSink::new_with_tenant(
        Arc::clone(store), ctx.tenant.to_string()
    )
    .write(None, kind, payload, principal_id_for_audit(req))
    .await { ... }
}
```

So the audit row, post-merge, carries:
- `kind = AuditKind::SkillCandidatePromoted` → serialised as
  `"skill.candidate.promoted"`
- `payload.endpoint = "worker.skills.promoted"`
- `payload.tenant_id = ctx.tenant.to_string()`
- `payload.worker_id = ctx.worker.id.to_string()`
- `payload.skill_id = skill_id.0.to_string()`
- `payload.promotion_kind = "learned_to_active"`
- `audit_record.tenant_id` column = `ctx.tenant.to_string()` (denorm
  via `new_with_tenant`)
- `audit_record.principal_id` column = result of
  `principal_id_for_audit(req)` (parses `X-Argo-Principal` header)

### Tests pin all 7 of those, byte-exact

Test 1 (`promoted_audit_row_payload_carries_skill_worker_promotion_kind_and_tenant`,
lines 183-233):
```rust
assert_eq!(kind, "skill.candidate.promoted");
assert_eq!(payload["endpoint"], Value::String("worker.skills.promoted".to_string()));
assert_eq!(payload["worker_id"], Value::String(worker.id.to_string()));
assert_eq!(payload["skill_id"], Value::String(skill_id.0.to_string()));
assert_eq!(payload["promotion_kind"], Value::String("learned_to_active".to_string()));
assert_eq!(payload["tenant_id"], Value::String(tenant.to_string()));
```

Test 2 (`promoted_audit_row_denorm_tenant_id_column_matches_workspace_tenant`,
lines 244-282):
```rust
assert_eq!(tenant_col, tenant.to_string(), ...);
assert_ne!(tenant_col, "__legacy__", ...);   // load-bearing
assert_ne!(tenant_col, "ws-default", ...);   // load-bearing
```

The two `assert_ne!` calls are the load-bearing guard: a refactor
swapping `new_with_tenant` for bare `new()` would land the row with
`__legacy__` or `ws-default` in the denorm column — and would flip
this test red instantly.

Test 3 (`promote_in_tenant_a_does_not_leak_into_tenant_b_audit_rows`,
lines 294-360):
```rust
let leaked: i64 = sqlx::query_scalar(
    "SELECT COUNT(*) FROM audit_record \
     WHERE kind = 'skill.candidate.promoted' AND tenant_id = ?",
).bind(tenant_b.to_string())...;
assert_eq!(leaked, 0, "tenant_b must see zero ...");

// VACUOUS-PASS GUARD: tenant_a's own query returns >=1
let owned: i64 = ...; // same query bound to tenant_a
assert!(owned >= 1, ...);
```

The vacuous-pass guard is a tell-tale of anti-theater discipline: a
buggy refactor that filtered out ALL `skill.candidate.promoted` rows
(or wrote with `tenant_id = NULL`) would pass `leaked == 0` trivially
but fail `owned >= 1`. Both halves are required.

Test 4 (`promote_audit_row_principal_id_column_is_stamped_when_caller_sends_x_argo_principal`,
lines 364+): pins `principal_id_for_audit(req)` extraction via the
`X-Argo-Principal` header — defends `mvp-audit-principal-stamp`.

### Sensible deviations are flagged honestly

The PR body acknowledges 2 deviations from the brief, both reasonable:

1. **Path**: brief named `crates/argo-gateway/tests/...`; that crate
   doesn't exist (gateway lives under `argo-cli/src/handlers/gateway/`).
   And `promote_response()` is `pub(super)`, so any test under
   `argo-cli/tests/` couldn't reach it. The PR followed the in-module
   `routes_worker_skills_tests.rs` pattern — same wire as the existing
   sibling test file.

2. **Test 4 substitution**: brief asked for a `FaultyAuditSink`
   injectable proving "ratchet must roll back" on audit-write failure.
   `helpers::emit_audit` constructs the sink inline (no injection seam),
   and the current contract is "audit is best-effort, ratchet is the
   source of truth" (ratchet writes BEFORE audit, in separate awaits).
   PR substituted: pin the principal-stamp seam instead. Same dimension
   of contract (audit-row column matches what the gateway extracted).

Both deviations are documented at the top of the test file's header
comment and re-explained in the PR body. This is the right level of
"call out what you did differently and why."

## Anti-theater check

- 7 distinct production seams pinned by 4 tests with 14 assertions.
- Vacuous-pass guard on test 3 (tenant_a query returns >=1 alongside
  tenant_b returns 0).
- `assert_ne!` against `__legacy__` and `ws-default` sentinels in test
  2 — pins the load-bearing `new_with_tenant` call, not just "sink fired."
- PR body honest about brief deviations (not silently quiet).
- 4-line wire-up (`#[cfg(test)] mod`) sits alongside an existing
  sibling test mod — small surface area, correct pattern.

## Gap / follow-up brief candidates

NONE NEW. This PR closes round-30's P2 finding cleanly. Two adjacent
contracts are now pin-able with the same test pattern and the same
helper:

1. The 7 other gateway routes that call `helpers::emit_audit` (line
   123, 168, 222, 262 in `routes_worker_skills.rs` alone) inherit the
   same audit-emit fence shape — a sweep-style follow-up
   `eval-gateway-emit-audit-callers-byte-exact-payload-sweep` (P3,
   ~400 LOC, 7 routes × 4 contract tests = 28 byte-exact tests) would
   make the entire helper surface self-defending. Lower priority — the
   helper is correct + the round-37 row-6 test pins the helper's
   payload-merge logic transitively.

2. The acknowledged "ratchet rollback on audit fail" gap (PR's test-4
   substitution) is its own follow-up: requires `AuditSink` injection
   seam + transaction boundary. Recommend
   `mission-promote-ratchet-and-audit-atomic-transaction` (P3, ~80-120
   LOC) — low priority because today the contract is "audit is
   best-effort, ratchet is source of truth," and no production caller
   relies on perfect 1-row-per-promote count.

## Mission claim impact

Claim 4 MVP row 6 (operator-approve flow + skill.candidate.promoted
audit emit) was 🟡 substrate-thin pre-PR; now 🟢🟢 e2e VERIFIED at
the gateway HTTP layer. With this PR and PR #2143 (lifecycle filter)
both merged, Claim 4 MVP score ≈ ~95% LIVE. The standalone
operator-approve flow at the memory layer was already 🟢🟢 from round
31c (the test file at `crates/argo-skill/tests/claim_4_mvp_row_6_operator_approve_emits_audit.rs`).
This PR closes the gateway-HTTP-layer half of the same row, completing
the operator-feedback loop end-to-end.
