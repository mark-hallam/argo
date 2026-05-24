# Round 50 — PR #2257 `eval-channels-cross-tenant-negative-test` PRE-MERGE anti-theater verify

- **Date**: 2026-05-24 (Melbourne 2026-05-25 06:1x AEST)
- **Round**: 50
- **Mission claim covered**: Claim 6 (Multi-channel + multi-tenant) — argo-channels substrate row in mission-vs-code gap manifest
- **Verdict**: 🟢 PASS · severity tier P1 (regression-pin only; no production change)

## Setup

PR head: `62b1648596ebb59625f462072586303f7987f47c`, branch `codex-eval-channels-cross-tenant-negative-test`.

```
git worktree add /tmp/pr-2257-verify -b verify-2257-r50 origin/codex-eval-channels-cross-tenant-negative-test
cd /tmp/pr-2257-verify
CARGO_TARGET_DIR=/tmp/pr-2257-verify-target cargo test -p argo-channels --test cross_tenant_test
```

Cold build: 11m 40s · test run: 0.19s.

## Observed

```
running 4 tests
test slack_inbound_cross_secret_refusal_should_emit_tenant_mismatch_audit ... ignored, FINDINGS-ONLY: argo-channels::SlackInbound has no AuditSink/tenant context; follow-up should wire tenant_mismatch=true audit in the gateway boundary
test slack_inbound_cross_secret_is_publicly_identical_to_bad_signature ... ok
test approval_token_cross_secret_is_publicly_identical_to_nonexistent_token ... ok
test outbound_dedup_is_namespaced_by_tenant ... ok

test result: ok. 3 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.19s
```

Diff: 2 files, +333 / -1 LOC. Production source `crates/argo-channels/src/**` untouched — pure regression-pin PR.

## Anti-theater check

PR body claims "production source under `crates/argo-channels/src/**` is untouched" — confirmed by `gh pr diff 2257`. The risk pattern for this shape of PR is **vacuously-true assertions** (tests that pass because they're testing the test scaffold rather than production behavior). I verified each load-bearing assertion against the actual SQL predicate or signature-verification path it pins.

**Assertion #1 — `outbound_dedup_is_namespaced_by_tenant`**:

```rust
let first_a = dispatcher_a.dispatch("slack", "slack-1", body, &[]).await?;
let first_b = dispatcher_b.dispatch("slack", "slack-1", body, &[]).await?;
let replay_b = dispatcher_b.dispatch("slack", "slack-1", body, &[]).await?;
assert_ne!(first_a.message_id, first_b.message_id);
assert_eq!(first_b.message_id, replay_b.message_id);
assert_eq!(transport.sent_count(), 2);
```

Grounded in `crates/argo-store/src/outbound_message_query.rs:118-123`:

```sql
SELECT ... FROM outbound_message
WHERE tenant_id = ? AND body_hash = ? AND sent_at >= ?
ORDER BY sent_at DESC LIMIT 1
```

If the `tenant_id = ?` predicate were missing, tenant_b's first dispatch would match tenant_a's already-cached row → `lookup_recent_by_body_hash` returns Some(prior) → dispatcher returns the cached outcome with `tenant_a's message_id` → `first_a.message_id == first_b.message_id` → test FAILS on the `assert_ne!`. Same-tenant replay (`first_b.message_id == replay_b.message_id`) is the symmetric positive control proving dedup still works inside-tenant.

**Assertion #2 — `approval_token_cross_secret_is_publicly_identical_to_nonexistent_token`**:

```rust
let after = get_request(store.pool(), &approval_id).await?.unwrap();
assert_eq!(after.status, ApprovalStatus::Pending, "tenant_b attempts must not mutate tenant_a's approval");
```

Grounded in `crates/argo-store/src/approvals.rs:117-136`:

```rust
pub async fn decide_request(pool, id, tenant_id: TenantId, grantor, decision) -> ArgoResult<bool> {
    sqlx::query("UPDATE approval_request SET ... WHERE id = ? AND tenant_id = ? AND status = 'pending'")
        .bind(...)
        .bind(tenant_id.0.as_bytes().as_slice())
        ...
}
```

When the test calls `decide_request(pool, &approval_id, tenant_b, ...)` with the cross-tenant token, the UPDATE matches 0 rows → returns `false` → the helper maps `false` to `Err(PUBLIC_NOT_FOUND)`. The assertion that `after.status == Pending` would FAIL on a hypothetical regression that dropped the `AND tenant_id = ?` predicate (cross-tenant UPDATE would silently succeed, mutating tenant_a's row to Granted).

The additional load-bearing assertion `wrong_tenant_valid_secret == nonexistent` proves that even a token signed with the **correct** secret-A but called via tenant_b's context must still 404 — i.e. token validation alone is not sufficient; the SQL predicate is the load-bearing check.

**Assertion #3 — `slack_inbound_cross_secret_is_publicly_identical_to_bad_signature`**:

```rust
let cross_tenant = slack_signature_boundary(&inbound_a, body, &cross_headers);    // signed with tenant_b's secret
let nonexistent = slack_signature_boundary(&inbound_a, body, &nonexistent_headers); // malformed signature
assert_eq!(format!("{cross_tenant:?}"), format!("{nonexistent:?}"));
assert_eq!(cross_tenant, Err(PUBLIC_NOT_FOUND));
```

Both calls map through `verify_signature(...).map_err(|_| PUBLIC_NOT_FOUND)`. The test would FAIL if `verify_signature` *accepted* a cross-tenant-signed payload (returning `Ok(())` makes `cross_tenant = Ok(())` ≠ `Err(PUBLIC_NOT_FOUND)`). So the assertion is load-bearing on real HMAC-SHA256 signature verification logic in `argo-channels::slack_inbound::sign_slack_v0` and its verifier.

The trailing positive control (sign with secret-A, verify with `inbound_a`, must succeed) prevents the symmetric regression: if `verify_signature` started rejecting **all** signatures, the test still wouldn't catch the cross-tenant case being silently accepted — the positive control closes that gap.

**Assertion #4 (FINDINGS-only, `#[ignore]`'d)** — `slack_inbound_cross_secret_refusal_should_emit_tenant_mismatch_audit`:

```rust
#[test]
#[ignore = "FINDINGS-ONLY: argo-channels::SlackInbound has no AuditSink/tenant context; ..."]
fn slack_inbound_cross_secret_refusal_should_emit_tenant_mismatch_audit() {
    let err: ChannelResult<()> = Err(ChannelError::Auth("Slack signature mismatch".into()));
    assert!(err.is_ok(), "desired post-fix shape: ...");
}
```

This is a deliberate placeholder. Marked `#[ignore]` so default `cargo test` doesn't run it; asserts `err.is_ok()` on a known-Err value so if a future engineer un-ignores it without wiring the audit, it'll fail loudly. Mission-vs-code gap-manifest row for `argo-channels` was updated in the same diff to add: *"plus Slack inbound tenant-mismatch audit emission at the gateway boundary"* under remaining follow-up.

## Gap / follow-up brief candidates

- **`mission-slack-inbound-tenant-mismatch-audit-emission` (P2, ~80-150 LOC)** — pinned by the #[ignore]'d FINDINGS-only test. Wire `tenant_mismatch=true` audit emission at the gateway boundary where `SlackInbound::verify_signature` is called (`SlackInbound` itself has no `AuditSink`/`TenantId` context, so the audit must happen one layer up). Brief should grep for the route handler in `argo-cli/src/handlers/gateway/routes_channels_slack_*.rs`. Not demo-blocking.

## Verdict signal for next conductor tick

**Recommend admin-merge next claude-conductor tick.** #2257 closes a regression-pin gap in claim-6 multi-tenant boundary coverage for the channels substrate. 30th anti-theater-honest verification this week. The 3 active tests are load-bearing on real SQL predicates / HMAC signature verification logic; would each FAIL on a regression that dropped tenant scoping at the data layer or accepted cross-tenant signatures at the protocol layer.
