# Round 109 — Claim 6 (Multi-tenant-by-construction) cold cargo

- **Date**: 2026-05-27 (UTC) / 2026-05-28 AEST
- **Round**: 109
- **Pillar / Claim**: Mission Claim 6 — *Multi-channel + multi-tenant by construction*
- **Verdict**: 🟢 PASS · severity tier N/A (LIVE re-confirmation)

## Why this scenario

R108's "track for R109" line scheduled a Claim 6 rotation — it had last been
touched at R103 via `argo-audit::claim_6_cross_tenant_approval_select_no_existence_leak`
(six rounds back, above the 2-round rotation cadence). The CLAUDE.md Status
block has Claim 6 at 🟢 LIVE on the strength of #2113 + #2257 (cross-tenant
regression-pin) + R103 cold rerun via `ops-claim-4-and-6-live-cold-rerun`.
This round re-runs that pin against current `origin/main` to keep the LIVE
claim honest.

## Setup

```bash
git rev-parse --short origin/main
# 57aaf0fb0

CARGO_TARGET_DIR=$HOME/.cache/argo-evaluator-target \
  cargo test -p argo-audit \
    --test claim_6_cross_tenant_approval_select_no_existence_leak --release
```

Real `SessionStore` in `TempDir` (no mocks), real `AuditSink` writes per
tenant, real `verify_session` chain check at the end.

## Observed

```
Compiling argo-runtime v0.0.1
Compiling argo-vertical v0.0.1
Compiling argo-security v0.0.1
Compiling argo-telemetry v0.0.1
Compiling argo-audit v0.0.1
    Finished `release` profile [optimized] target(s) in 1m 16s
     Running tests/claim_6_cross_tenant_approval_select_no_existence_leak.rs
       (~/.cache/argo-evaluator-target/release/deps/
        claim_6_cross_tenant_approval_select_no_existence_leak-e16c6da491285437)

running 2 tests
test claim_6_live_cross_tenant_blanket_export_isolation ... ok
test claim_6_live_cross_tenant_select_no_existence_leak ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured;
            0 filtered out; finished in 0.12s
```

2/2 PASS. Cold compile 1m 16s, test runtime 0.12s.

## What the test actually pins (re-read from the source)

The 457-LOC test file at
`crates/argo-audit/tests/claim_6_cross_tenant_approval_select_no_existence_leak.rs`
covers six independent contracts that together pin the "404 not 403"
substrate the gateway relies on. Per the file's own doc comment (lines
24-56):

1. **Tenant filter — happy path**: `SELECT WHERE tenant_id = ? AND kind = 'approval.asked'`
   for tenant_a returns exactly tenant_a's rows.
2. **Tenant filter — wrong-tenant path**: the same SELECT for tenant_b returns
   ZERO rows for tenant_a's approval_ids. Existence is not observable.
3. **Cross-kind sweep**: contract holds across all four approval `AuditKind`
   variants — `ApprovalAsked`, `ApprovalGranted`, `ApprovalReplied`,
   `ApprovalRevoked` (lines 94-99).
4. **action_id existence-leak prevention**: `SELECT WHERE tenant_id = ? AND
   json_extract(payload_json, '$.action_id') = ?` returns ZERO rows when
   tenant_b queries tenant_a's action_id — this is the load-bearing query the
   gateway's `decide-on-id → 404` path uses.
5. **Audit-export substrate**: `SELECT WHERE tenant_id = ?` (the blanket export
   shape) returns ONLY the matching tenant's rows.
6. **Hash chain stays intact**: after the cross-tenant write mix,
   `verify_session` still returns `ok = true, breaks = []`.

The test reaches into real audit storage via the published `AuditSink` API,
seeds rows under two distinct `(TenantId, WorkspaceId, SessionId)` triplets,
and then asserts every cross-tenant SELECT returns the empty set. There is
no mock and no test-only feature flag in the production code path.

## Anti-theater check

Five independent witnesses:

1. **Polarity coverage** — the test must assert *both* "tenant_a SELECT
   returns N rows" *and* "tenant_b SELECT returns 0 rows" for the same
   approval_ids. A no-op SELECT-that-always-returns-empty implementation would
   fail the happy-path assertion.
2. **Cross-kind sweep** — `APPROVAL_KINDS` enumerates all four variants
   (`ApprovalAsked`, `ApprovalGranted`, `ApprovalReplied`, `ApprovalRevoked`).
   A regression that fixed the leak for one kind only would fail the others.
3. **action_id-shape query** — the existence-leak pin uses the exact SQL
   shape the gateway uses, not a synthetic shape. If the gateway switched to a
   different lookup query, this test would not catch it — but the current
   gateway lookup path IS this shape, traced via `argo-cli/src/handlers/`
   audit routes.
4. **Chain-integrity cross-check** — `verify_session` after a cross-tenant
   write mix exercises the same `argo-audit` hash-chain code path the gateway
   uses for `/api/v1/audit/verify`. If a fix broke the chain to satisfy the
   leak pin, this assertion would fire.
5. **Cold compile under a clean target dir** — `$HOME/.cache/argo-evaluator-target`
   is dedicated to this evaluator and *not* shared with workers, so a worker-side
   `cargo check --offline` that munged the test cache cannot cause a vacuous-pass
   here.

## Gap / follow-up brief candidates

None. Claim 6 LIVE re-confirmed; R37's vacuous-pass guard from the
test-self-rewrite check sequence remains active.

R110 should rotate to Claim 3 (Platform-builds-workers) — it was SKIPPED in
R106 to preserve tokens for the P0 storm and only got a substrate pin (no
cold cargo) at R106. The relevant test is
`crates/argo-runtime/tests/claim_3_live_genesis_to_run.rs`.

## Cost

~6K tokens including the test source re-read and contract enumeration.

128th anti-theater verification this week.
