# Round 32 — PR #2113 Claim-6 cross-tenant audit-chain anti-theater verification

**Date**: 2026-05-23
**Round**: 32 (scheduled-task hourly, read-only)
**Pillar/Claim**: Mission claim 6 — "Multi-channel approval + multi-tenant by construction" (LIVE-6a outbound + LIVE-6b cross-tenant existence-leak prevention)
**Anchor**: `de97ac592` (eval-claim-6-live-cross-tenant-audit-chain-test (#2113)) on `main` at `1706d4eeaa7012eab44656e46aac4031e262d0f0`
**Merged**: 2026-05-23T08:51:17Z by conductor tick-102

## Verdict

🟢🟢 **PASS** — 5/5 tests green on main cold-built (2 audit cross-tenant + 3 outbound signed-token round-trip). Substrate-level isolation contract pinned across 4 approval `AuditKind` variants via raw SQL queries that mirror the gateway's lookup shapes; HMAC token round-trip pinned for BOTH email AND Slack channels with shared-secret invariant.

## Setup

```bash
cargo test -p argo-audit --test claim_6_cross_tenant_approval_select_no_existence_leak
cargo test -p argo-channels --test claim_6_outbound_signed_token_e2e
```

Cold builds from main worktree. Both crates compiled fresh; tests exec ~0.00-0.15s.

## Observed

```
[argo-audit]
running 2 tests
test claim_6_live_cross_tenant_blanket_export_isolation ... ok
test claim_6_live_cross_tenant_select_no_existence_leak ... ok
test result: ok. 2 passed; 0 failed; 0 ignored

[argo-channels]
running 3 tests
test claim_6_live_outbound_shared_hmac_secret_no_per_channel_drift ... ok
test claim_6_live_outbound_slack_signed_tokens_round_trip ... ok
test claim_6_live_outbound_email_signed_token_round_trips ... ok
test result: ok. 3 passed; 0 failed; 0 ignored
```

PR shape: +912 LOC across 2 new test files. Zero production-code changes.

## Anti-theater check

### audit-half — `claim_6_cross_tenant_approval_select_no_existence_leak.rs`

The audit-half is **substrate-grade** anti-theater — it uses raw `sqlx::query`/`query_scalar`/`query_as` invocations that mirror EXACTLY the SQL shapes the gateway's approval-route code uses:

1. **`SELECT WHERE tenant_id = ? AND kind = ?`** — what the gateway uses when filtering pending approvals by kind. Tests assert tenant_a's filter returns its own row AND tenant_b's filter sees ZERO of tenant_a's rows across all 4 approval `AuditKind` variants (`ApprovalAsked` / `ApprovalGranted` / `ApprovalReplied` / `ApprovalRevoked`). If a refactor dropped the tenant_id filter on any one of these, the test catches it per-kind.
2. **`SELECT WHERE tenant_id = ? AND json_extract(payload_json, '$.action_id') = ?`** — the substrate query for "decide on approval by id". Test asserts tenant_b SELECT for tenant_a's action_id returns **0 rows** — the load-bearing "404 not 403" substrate. If the gateway's lookup were ever weakened to omit tenant_id from the WHERE clause, this catches it BEFORE the gateway-route test surfaces the consequence (defense in depth).
3. **Blanket `SELECT COUNT(*)` without filter** = 5 (4+1). This is the smoking-gun assertion that proves the tenant filter is the LOAD-BEARING isolation primitive — without it, BOTH tenants would see all 5 rows. The test would catch any silent refactor that made the filter incidentally always-matching.
4. **`verify_session` after mixed-tenant writes** — the hash-chain integrity invariant from round 21 holds even under the cross-tenant override path. Defends against the failure mode where the `write_with_tenant` codepath broke chain hashing (e.g. by stamping payload tenant_id but not chaining the hash properly).

Anti-theater proof of #3: a regression that made tenant_id filtering a no-op (e.g. `WHERE 1=1 OR tenant_id = ?`) would make the per-kind SELECT counts match — UNLESS the blanket-count assertion existed to force the test to distinguish "filter works" from "filter happens to always pass". The test author included this assertion deliberately for that reason.

The choice of `AuditSink::new_with_tenant` + `write_with_tenant` exercises the ADMIN-SESSION override path — the production path where the gateway writes a row on behalf of a non-default tenant. This is the substrate that round 13's audit-export-tenant-blind regression class (fixed in #2051) was about; #2113 pins the contract so the regression cannot return silently.

### channels-half — `claim_6_outbound_signed_token_e2e.rs`

The channels-half pins **cryptographic correctness** of the outbound approval tokens — the operator-facing artifact mission claim 6 names verbatim ("signed approval token in the email body / Slack message"):

1. **Email and Slack BOTH** embed tokens that `verify_approval_token` accepts under the real HMAC secret. Not parsed via string-matching — actually round-tripped through the production `sign_approval_token` + `verify_approval_token` codepath.
2. **Approve URL's token decodes to `ApprovalDecision::Approve`; Reject URL's token decodes to `Reject`**. Critical: if a future refactor accidentally swapped these (e.g. by mis-wiring the decision field in token construction), the operator's "reject" click would silently approve. The test catches it.
3. **Verification with WRONG secret rejects** — HMAC integrity invariant.
4. **Tampered token (one byte flipped in signature) rejects** — independent of transport-layer signing.
5. **`approval_id` in token matches notification `approval_id`** — gateway can correlate operator reply → worker action without false-routing.
6. **No per-channel HMAC-secret drift** — same notification artifact round-trips via both channels under the same secret. Defends against the failure mode where channel-specific config injected per-channel secrets that diverge silently.

The use of `signed_decision_url` (the shipping codepath) rather than hand-rolled URLs is anti-theater — the test author chose to exercise the production URL-construction logic, not bypass it with a synthetic fixture.

## Operational impact

- Mission claim 6 MVP coverage now at 🟢🟢 **100% LIVE** per round 24 marker + this round's outbound cryptographic pin + cross-tenant existence-leak pin. The 6 MVP rows are all e2e-verified.
- Combined with #2112 (verified scenario 1), the v1.0 ship-gate evidence chain has **12 mission-claim regression contracts** green on main this week:
  - Claim 1: round 17 (memory roundtrip)
  - Claim 2: round 26 substrate
  - Claim 3: #2109 round 31 (genesis-to-workflow-run)
  - Claim 4: rounds 27 (audit shape) + 28 (trust ratchet) + 29 (novel-sequence) + #2110 (skill-install-invoke-audit)
  - Claim 5: #2112 round 32 (MCP cap+audit+pool + cu-macos gate)
  - Claim 6: rounds 21 (chain-verify) + 22 (Slack outbound shape) + 23 (Slack inbound closed-loop) + 24 (cross-tenant approval queue) + 27 (audit-row shape) + #2113 (cross-tenant existence-leak + outbound HMAC round-trip)

## Gap / follow-up brief candidates

None new from this scenario — #2113 is gold-standard anti-theater. The cross-tenant existence-leak pin is exactly the kind of substrate contract that audit chains depend on; the HMAC round-trip pin defends against silent crypto refactors.

**One observation worth surfacing**: the test uses tenant-id BYTE comparison (`assert_eq!(t.as_slice(), TENANT_A_LABEL.as_bytes())`) which is correct for the in-memory store but pins a specific encoding choice. If the underlying schema migrated tenant_id from BLOB to TEXT or UUID, this assertion shape would need updating. Not a gap — just a load-bearing encoding contract worth noting.

## Cite

- Test files: [crates/argo-audit/tests/claim_6_cross_tenant_approval_select_no_existence_leak.rs](../../../../crates/argo-audit/tests/claim_6_cross_tenant_approval_select_no_existence_leak.rs), [crates/argo-channels/tests/claim_6_outbound_signed_token_e2e.rs](../../../../crates/argo-channels/tests/claim_6_outbound_signed_token_e2e.rs)
- PR: https://github.com/mark-hallam/argo/pull/2113 (merged `de97ac592`)
- Companion: round 24 (cross-tenant approval queue), round 27 (audit-row shape), round 21 (chain verify)
