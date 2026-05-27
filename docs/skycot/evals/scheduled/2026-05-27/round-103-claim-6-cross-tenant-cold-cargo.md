# Round 103 — Mission Claim 6 (Multi-tenant by construction) — cross-tenant approval no-existence-leak LIVE cold cargo

- **Date**: 2026-05-27 (UTC; AEST: 2026-05-28 ~00:30 local)
- **Round**: 103
- **Pillar / Claim**: Axis 2 — mission claim coverage. Claim 6 = Multi-channel + multi-tenant.

## Verdict
🟢 **PASS** — Claim 6 substrate LIVE on current `origin/main`. P0/P1 unchanged.

## Setup
```bash
git rev-parse --short origin/main          # 9f8bb0b98 (head)
cargo test -p argo-audit \
  --test claim_6_cross_tenant_approval_select_no_existence_leak --release
```

Test file: `crates/argo-audit/tests/claim_6_cross_tenant_approval_select_no_existence_leak.rs` (457 LOC, 2 `#[tokio::test]` functions).

## Observed
```
   Compiling argo-audit v0.0.1 (~/code/argo/crates/argo-audit)
    Finished `release` profile [optimized] target(s) in 1m 02s
     Running tests/claim_6_cross_tenant_approval_select_no_existence_leak.rs
running 2 tests
test claim_6_live_cross_tenant_blanket_export_isolation ... ok
test claim_6_live_cross_tenant_select_no_existence_leak ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;
finished in 0.13s
```

Cold cargo: **62s compile + 0.13s test runtime**. 2/2 PASS against `origin/main` head `9f8bb0b98`.

## What the tests actually exercise

`claim_6_live_cross_tenant_select_no_existence_leak` (lines 161-354) — five real assertions:

1. **Seed real rows via real `AuditSink::write_with_tenant`** for all four approval `AuditKind` variants (`ApprovalAsked`, `ApprovalGranted`, `ApprovalReplied`, `ApprovalRevoked`) — one row per kind per tenant (8 rows total in same chain).
2. **Kind-and-tenant filter — happy + wrong path.** `SELECT WHERE tenant_id=? AND kind=?` for tenant_a returns its own row; for tenant_b filtered by `action_id LIKE 'act-a-%'` returns 0. Pin: cross-tenant column isolation across ALL 4 approval kinds (not just `ApprovalGranted`).
3. **Action_id existence-leak prevention** — sanity-check that tenant_a sees its own `act-a-*` action_id (a=1 row), then load-bearing assertion: tenant_b's SELECT for the same `act-a-*` action_id returns 0. This is the substrate that lets the gateway return 404-not-403 on cross-tenant decide-by-id.
4. **Audit-export shape pin** — `SELECT WHERE tenant_id=? ORDER BY seq ASC` returns exactly tenant_b's 4 rows; every payload's `action_id` starts with `act-b-`; none start with `act-a-`; `payload['tenant_id']` matches the column. Pins #2051's `POST /api/v1/audit/export` fix.
5. **Hash chain holds under mixed-tenant writes.** `verify_session` returns `ok = true`, `rows = 8`, `breaks = []`.

`claim_6_live_cross_tenant_blanket_export_isolation` (lines 356-457) — defence-in-depth: seeds 4 `ToolCall` rows under tenant_a + 1 `ApprovalAsked` under tenant_b, asserts tenant_a's export = 4 rows (all `ToolCall`) and tenant_b's export = 1 row (`ApprovalAsked`), then asserts the blanket-without-filter `SELECT COUNT(*) WHERE session_id=?` returns 5. The last assertion is the load-bearing anti-theater — it proves the tenant filter is genuinely the isolating primitive (rows ARE all in the table; only the filter keeps them separate).

## Anti-theater check

- **The tests query the real `audit_record` table via `sqlx::query`/`query_scalar` against the real `SessionStore::pool()`** — no mocked store, no in-memory DSL wrapper.
- **The "sanity that tenant_a sees its own row"** check (`a_sees_own == 1`) defends against a class of false-positive where both queries return 0 because the seed never landed. Both halves of the negative-existence assertion are anchored.
- **The blanket count = 5** check in the second test proves the per-tenant SELECTs aren't accidentally always-empty — the filter is the load-bearing isolation.
- **Hash-chain `verify_session` PASS under mixed-tenant writes** rules out the failure mode where cross-tenant overlay paths corrupt the chain.
- **Gateway production code corroborates** — `crates/argo-cli/src/handlers/gateway/routes_audit_detail.rs:107` carries `WHERE (tenant_id = ? OR (? = ? AND tenant_id = ?))` on the detail lookup, and `argo-store` carries 15+ `WHERE tenant_id = ?` SELECTs across SCIM, worker, roster, workflow_test_run, skill_share, tenant_rate_limit, workspace_keys, worker_budget_write, etc. The Claim 6 substrate is plumbed beyond just the audit substrate.

## 111th anti-theater verification this week
Claim 6 multi-tenant isolation re-confirmed on current `origin/main`. The 2026-05-25 cold-rerun reported in CLAUDE.md (52.42s compile + 0.13s runtime via `ops-claim-4-and-6-live-cold-rerun-on-current-main`) ran a *different* claim-6 test (`claim_6_live_cross_tenant`); this round broadens coverage by running the audit-substrate isolation that pins the 4-kind sweep + action_id existence-leak + audit-export shape contracts.

## Gap / follow-up brief candidates
**None.** Claim 6 LIVE confirmed. Status block in CLAUDE.md correctly marks it 🟢 LIVE.

## Source
- Test: `crates/argo-audit/tests/claim_6_cross_tenant_approval_select_no_existence_leak.rs`
- Gateway tenant filter: `crates/argo-cli/src/handlers/gateway/routes_audit_detail.rs:107`
- MISSION-locked criteria: `docs/skycot/MISSION.md` Claim 6 + round-18 LIVE criteria.
- Earlier rounds: R102 ran Claim 2 (LIVE PASS); R96/R94 ran Claim 4. Claim 6 hadn't been touched in ≥3 rounds.
