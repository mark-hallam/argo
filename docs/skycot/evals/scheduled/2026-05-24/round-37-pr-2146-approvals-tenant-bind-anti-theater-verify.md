# Round 37 · PR #2146 — `mission-approval-decide-request-tenant-pinned` anti-theater verify

- **Date**: 2026-05-24
- **Round**: 37 (scheduled-task hourly fire, read-only)
- **Pillar / Claim**: Claim 6 — Multi-tenant by construction (substrate)
- **PR**: [#2146](https://github.com/mark-hallam/argo/pull/2146) (MERGED, commit `d3e626698`)
- **Verdict**: 🟢🟢 **PASS** — GOLD-STANDARD ANTI-THEATER
- **Severity if it had failed**: P1 (latent cross-tenant decide leak)

## Setup

```bash
git log --oneline origin/main -25
git show --stat d3e626698
grep -n 'fn decide_request' crates/argo-store/src/approvals.rs
grep -n 'tenant_id' crates/argo-store/src/approvals.rs
rg -n 'decide_request' crates/ --type rust
cargo test -p argo-store --test approval_decide_tenant_bind_pin
```

## What the PR changes

Closes §4-finding-1 from PR #2116 (`eval-argo-store-tenant-bind-sweep`):
production `approvals::decide_request` previously filtered on
`(id, status)` only — the same SQL-bind shape that produced the
`audit_detail` cross-tenant bleed fixed in #1992. Pre-this-PR, the
helper was "safe today only because no production route wires it" —
the leak materialised the instant one did.

Diff:
- `crates/argo-store/src/approvals.rs:117-141` — `decide_request`
  signature gains `tenant_id: TenantId`; UPDATE WHERE clause now binds
  `id = ? AND tenant_id = ? AND status = 'pending'`.
- `crates/argo-store/tests/approval_decide_tenant_bind_pin.rs` —
  3 active tests (no `#[ignore]`):
  - `decide_request_rejects_cross_tenant_decide` (the FINDINGS-ONLY
    test from #2116, un-ignored + assertion inverted)
  - `same_tenant_decide_still_succeeds` (positive pin)
  - `cross_tenant_reject_does_not_block_legitimate_decide`
    (rejection-is-real pin)
- 3 callsite rebases: `approvals_tests.rs`, `approvals_tests_expire.rs`,
  `approval_escalation_tests.rs`, `claim_6_cross_tenant_approval_queue.rs`.

## Observed (anti-theater proof)

### 1. Production code change is load-bearing

Direct read of `crates/argo-store/src/approvals.rs:117-141`:

```rust
pub async fn decide_request(
    pool: &SqlitePool,
    id: &str,
    tenant_id: TenantId,           // ← NEW
    decided_by: &str,
    decision: ApprovalDecision,
) -> ArgoResult<bool> {
    let id_bytes = parse_id_bytes(id)?;
    let now = Utc::now().timestamp_micros();
    let rows = sqlx::query(
        "UPDATE approval_request \
         SET status = ?, decided_at = ?, decided_by = ?, decision = ? \
         WHERE id = ? AND tenant_id = ? AND status = 'pending'",  // ← NEW
    )
    .bind(decision.as_store_str())
    .bind(now)
    .bind(decided_by)
    .bind(decision.as_store_str())
    .bind(&id_bytes)
    .bind(tenant_id.0.as_bytes().as_slice())                       // ← NEW
    .execute(pool)
    .await
    ...
```

This is the real SQL bind. A wrong-tenant caller now hits `WHERE` with
a non-matching `tenant_id` bind → `rows_affected() == 0` → returns
`false` (the same "row already decided" idempotency path), leaving the
target tenant's row Pending and unmodified.

### 2. Tests pin the contract by behaviour, not by syntax

Test 1 (`decide_request_rejects_cross_tenant_decide`): hires worker
under tenant_a, creates approval row, calls `decide_request` from
tenant_b's principal. Asserts:
- Return is `false` (no row updated)
- `get_request` confirms row remains in `Pending` status
- `list_pending(tenant_a)` still contains the row

A regression that re-removed the `tenant_id` bind would flip
`rows_affected()` to 1 → return `true` → flip the row to Approved →
make this test fail loudly. The assertion shape isn't "compiled with
arg X" — it's "row state is unchanged in the DB after the call." That
is the actual safety property.

Test 2 (positive pin) defends against the inverse over-correction:
a refactor that added a stray `AND 0 = 1` would pass test 1 vacuously
but fail test 2.

Test 3 (rejection-is-real): tenant_b rejects → tenant_a then approves
→ asserts the row is Approved (not Rejected). Defends against a
refactor that "passes" by silently coercing all decisions to a single
status.

### 3. cargo verification (cold-built from origin/main)

```
running 3 tests
test cross_tenant_reject_does_not_block_legitimate_decide ... ok
test same_tenant_decide_still_succeeds ... ok
test decide_request_rejects_cross_tenant_decide ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### 4. Production caller audit — still zero (premise unchanged)

`rg -n 'decide_request' crates/ --type rust` returns 19 hits across 7
files — every one is either:
- The definition / docstring in `approvals.rs` (5 hits)
- The test file under `approvals_tests.rs` + `approval_escalation_tests.rs` + `approvals_tests_expire.rs` (helper-rebased callers)
- The new pin test in `tests/approval_decide_tenant_bind_pin.rs`
- The Claim 6 cross-tenant approval queue test in `tests/claim_6_cross_tenant_approval_queue.rs`

ZERO production callers (no gateway route, no CLI handler, no
orchestrator path). The brief's premise stands: the leak was latent.
This PR ensures it stays latent forever — any future wire-up inherits
the tenant-bound contract for free.

## Anti-theater check

- Production SQL really has the bind (line 129).
- Production fn signature really requires `tenant_id` (line 120).
- Test invokes `decide_request` from a tenant_b context and asserts
  row state, not return value alone — this is behaviour-pinning.
- cargo cold-built + ran clean (4m 59s compile, 0.27s test execution).
- The fence test (#2116's `#[ignore]`'d FINDINGS-ONLY) is now active
  and inverted — exactly the round-33 brief recommendation pattern
  the conductor adopted.

## Gap / follow-up brief candidates

NONE NEW. This PR closes the round-33 + round-35 systemic finding
cleanly. The natural follow-on is the `eval-gateway-uuid-bound-routes-tenant-ownership-sweep`
brief (P1, ~150-200 LOC) recommended round 35 — round 35 noted that
`mission-approval-decide-request-tenant-pinned` (now MERGED) +
`mission-gateway-session-export-tenant-bind` (TODO per tick-110)
are two instances of the same pattern; a sweep brief enumerating
every gateway route binding on UUID-alone without tenant ownership
pre-flight would catch the third before it ships.

Re-surfaced for visibility: `mission-gateway-session-export-tenant-bind`
(P0/P1 per tick-110, the second of two parallel cross-tenant SQL-bind
leaks identified in #2130).
