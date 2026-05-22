# mission-hire-audit-payload-shape-validation

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `6278d31c1` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-hire-audit-payload-shape-validation`.

🟡 **P1 SECURITY HYGIENE — PII redaction never proven.** Source: evaluator round-2 (`docs/skycot/evals/scheduled/2026-05-22/teammate-hire-five-distinct-personas.md`):

> "`mission-hire-audit-payload-shape-validation` (P1): Verify the WorkerHired row actually contains the un-redacted worker_id, redacted problem text (no PII), and actor principal_id. This eval did not inspect the row directly — `sqlite3` doesn't work (SQLCipher) and there's no admin endpoint to read raw audit_record rows."

Context: every hire flow writes a `WorkerHired` row to `audit_record` with a payload blob. The payload IS supposed to contain `{worker_id, persona_name, actor_principal_id, redacted_problem_text}` per the `Auditable` impl on the hire handler. **But no test asserts the actual shape.** If a regression silently drops `actor_principal_id` or fails to redact PII in `problem_text`, no one notices until a compliance audit.

## Scope

Add a regression test that:

1. Drives a hire e2e from the test gateway harness (existing `routes_workers_hire_test.rs` or sibling).
2. Provides a deliberately PII-rich `problem_text` input: `"Hire me a marketing manager. My email is alice@acmeco.com and my phone is +1-555-0123 and I work at ACME Corp at 123 Main St, San Francisco."`
3. Calls `POST /api/v1/workers` with that payload.
4. **Reads the `audit_record` row directly** from the `argo-store` SessionStore (test seam — bypass HTTP because there is no admin read-back endpoint).
5. Asserts on the JSON payload:
   - `worker_id` is the canonical UUID, **un-redacted** (e.g. `"550e8400-e29b-41d4-a716-446655440000"`).
   - `persona_name` matches the assembled draft.
   - `actor_principal_id` is **present + non-null** + matches the test fixture's principal.
   - `problem_text` is **redacted** — does NOT contain `alice@acmeco.com`, does NOT contain `+1-555-0123`, does NOT contain `123 Main St`. Specifically: assert presence of the redaction markers (`[EMAIL]`, `[PHONE]`, `[ADDR]` or whatever `argo-redact` uses).
6. **Cross-tenant negative**: same e2e but with tenant_b's cookie; assert the row's `tenant_id` column is tenant_b, NOT tenant_a (proves multi-tenant isolation extends into audit shape).

### How to read the audit_record from the test

`crates/argo-store/src/audit_store.rs` has `AuditStore::list_for_tenant(tenant_id, kind_filter, limit)`. The test imports the trait and calls it directly. This is in-process; no HTTP round-trip needed.

```rust
let audit_store = test_store.audit_store();
let rows = audit_store.list_for_tenant(&tenant_id, Some(AuditKind::WorkerHired), 10).await?;
let payload: serde_json::Value = serde_json::from_str(&rows[0].payload_json)?;
assert!(payload["worker_id"].as_str().unwrap().starts_with("550e8400"));
assert!(!payload["problem_text"].as_str().unwrap().contains("alice@acmeco.com"));
```

(If `audit_store()` doesn't exist on the test store, use whatever the existing audit tests use as the entry point.)

### What this test catches

- Regression: someone changes the `Auditable` impl on the hire handler to drop a field. The test fails loudly.
- Regression: someone wires raw `problem_text` into the audit payload without running it through `argo-redact`. The test fails on the PII assertion.
- Regression: someone changes the `argo-redact` patterns and `[EMAIL]` becomes `[REDACTED:email]` — test fails, forcing a contract update.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_workers_hire_tests.rs` (or wherever the hire e2e tests live — find with `rg "fn .*hire" crates/argo-cli/src/handlers/gateway/`).
- A NEW sibling test file `crates/argo-cli/src/handlers/gateway/routes_workers_hire_audit_payload_tests.rs` is preferable if the existing file is near 500 LOC.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- The `Auditable` impl on the hire handler — this brief asserts the existing behavior, doesn't change it.
- `crates/argo-redact/src/**` — the redactor itself. If the test reveals a bug in the redactor, that's a follow-up brief.
- `crates/argo-audit/src/event.rs` — the canonical AuditKind enum.

## Acceptance criteria

- [ ] New test (or pair of tests) asserts: WorkerHired payload has `worker_id`, `persona_name`, `actor_principal_id` populated; `problem_text` is PII-redacted.
- [ ] Cross-tenant negative test asserts: tenant_b's hire writes to tenant_b's audit_record, not tenant_a's.
- [ ] Tests pass with `cargo test -p argo-cli routes_workers_hire_audit_payload`.
- [ ] If the test reveals that the current production code does NOT redact PII → STOP, do not "fix" inline. Open the PR with the failing tests + a FINDINGS-ONLY note. Conductor authors a fix-brief.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `mission-hire-audit-payload-shape-validation`
- PR title: `test(gateway): WorkerHired audit-payload shape + PII redaction + cross-tenant (mission-hire-audit-payload-shape-validation)`

## Notes for the implementer

- ~100-180 LOC expected. 2 tests at ~50-70 LOC each + test helper imports.
- **The PII assertion is the load-bearing piece.** The whole point is to lock in "we redact, and we can prove it". If the assertion is just "redacted text != raw text", that's too weak — assert specific PII tokens are absent + redaction markers are present.
- **If you find that current production code DOES NOT redact** — that's a P0 finding. Open the PR with `#[ignore]`'d tests + a "FINDINGS-ONLY" PR-body note. Do NOT fix inline (out of brief scope; redactor wiring is a different surface).
- **The cross-tenant negative test reuses the pattern** from `routes_audit_feed_tests.rs` (cross-tenant existence checks). Find the existing test fixture and clone the setup.
- Cite the evaluator report verbatim in the PR body.
