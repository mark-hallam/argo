# round 45 — PR #2131 anti-theater verify (pre-merge)

- **Date**: 2026-05-23T22:27Z (Melbourne local Sun 2026-05-24 08:27 AEST)
- **Round**: 45
- **Pillar / Claim**: Claim 6 (Multi-tenant-by-construction) — orchestrator cross-tenant handoff audit visibility
- **PR**: [#2131](https://github.com/mark-hallam/argo/pull/2131) `mission-orchestrator-cross-tenant-handoff-audit-kind`
- **Verdict**: 🟢🟢 **PASS** — non-trivial substrate work; tests pass cold; pre-existing existence-leak contract preserved. Severity tier: P2 (claim-6 dashboard pin closure).

## Why this PR matters

This row has been demote-cycled 4 times across tick-115 → tick-126 → tick-129 → tick-131 (over 24h+). At round-45 start it was finally MERGEABLE at headRefOid `9072952b2`, but tick-131 demoted it again because of doc-tree clobber (auditor `T21-41.md`, evaluator round-44 overlay-recheck report, peer codex heartbeats). The **production substrate** has stayed stable across all the demote cycles, so verifying its contract pre-merge means the next worker who cleans up the doc-tree noise can safely ship the substrate without re-verification.

## Setup

```bash
git fetch origin pull/2131/head:pr-2131-verify
git checkout pr-2131-verify
cargo test -p argo-orchestrator --test cross_tenant_handoff_audit_test
cargo test -p argo-orchestrator --test cross_tenant_handoff_test  # sibling — existence-leak contract
```

Branch state at verify: `mergeable=MERGEABLE`, `state=OPEN`, head `9072952b2`, updatedAt `2026-05-23T22:13Z`. PR diff: 5 production files + 8 doc-tree files (the doc-tree noise is what tick-131 demoted on; substrate untouched by the noise).

## Observed

### Production code layer (`crates/argo-orchestrator/src/persist_handoffs.rs`)

`insert_handoff` now invokes `try_audit_cross_tenant_handoff_attempt` BEFORE returning the byte-identical `Schema("handoff session not found")` error on the cross-tenant refusal path. Order matters — the audit row writes to the **victim** tenant's chain even though the **caller** still receives the existence-leak-safe NotFound error.

`find_session_owner_tenant` is a NEW helper at `:370-389` that runs `SELECT tenant_id FROM session WHERE id = ? AND provider = ?` — caller-tenant-FREE lookup. Critically, this is only invoked **on the refusal path**, AFTER the standard tenant-pinned `find_session(tenant_id, session_id)` returned None. So the cross-tenant SELECT cannot itself leak — it's executed only when the caller has already proven they don't own the session.

Defence-in-depth at `:412-419`: `if victim_tenant_id == attempted_tenant_id { return Ok(()); }` — guards against a future refactor that loosens `find_session` and produces a self-attempt row. Honest rustdoc explains the branch is unreachable today but the guard exists for refactor safety.

`canonical_cross_tenant_payload` (`:436-460`) emits the 10-field payload spec'd in the PR body: session_id, attempted_tenant_id, victim_tenant_id, from_worker, to_kind, to_worker_id?, to_addr?, reason, **payload_hash** (sha256 hex via `argo_orchestrator::persist_audit::payload_hash` — no payload contents leaked), attempted_at. The payload-hash convention is identical to the pattern used by existing audit emission sites.

`write_cross_tenant_attempt_audit` (`:464-487`) computes `argo_audit::compute_chain_hash` against the parent hash + INSERTs into `audit_record` with the victim's session_id binding — the row lands on the victim chain, not the attempted-tenant chain, so victim-tenant operators see the leak attempt in `argo audit show`.

### AuditKind enum (`crates/argo-audit/src/event_kind.rs:487-497`)

```rust
HandoffCrossTenantAttempt
```
Variant added with rich docstring documenting the existence-leak contract preserved + payload shape + emission semantics. event.rs `label()`/`from_label()`/`all()` matches extended (additive). `event_tests.rs` variant-count assertion bumped 70 → 71 (the +1 accounts for `ApprovalDeclined` which landed on main between this branch's prior attempt and current rebase — honestly resolved in the rebase note).

### Test layer (`crates/argo-orchestrator/tests/cross_tenant_handoff_audit_test.rs` — NEW, 164 LOC)

```
running 3 tests
test nonexistent_session_insert_handoff_emits_no_audit_row ... ok
test cross_tenant_insert_handoff_emits_audit_row_on_victim_session ... ok
test cross_tenant_emission_does_not_change_public_error_shape ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s
```

The three tests pin the three load-bearing semantics:

1. **`cross_tenant_insert_handoff_emits_audit_row_on_victim_session`** — creates tenant_a session, tenant_b crafts handoff against it, asserts `err.is_err()` AND `rows == 1` AND payload[session_id/attempted_tenant_id/victim_tenant_id/to_kind/reason/attempted_at] AND `payload_hash.len() == 64` (32-byte sha256 hex, no payload contents leaked).
2. **`nonexistent_session_insert_handoff_emits_no_audit_row`** — fabricated ghost session id, asserts `err.is_err()` AND `rows == 0` — silence on genuine typos, no audit-chain pollution.
3. **`cross_tenant_emission_does_not_change_public_error_shape`** — asserts `format!("{cross_err:?}") == format!("{nonexistent_err:?}")` — byte-identical Debug format, regression guard for the existence-leak contract pinned by the sibling file.

### Sibling existence-leak contract (`crates/argo-orchestrator/tests/cross_tenant_handoff_test.rs`)

13/13 PASS in 1.26s (cold) — the existing cross-tenant tests pinning the byte-identical-NotFound contract still pass. Critically includes `insert_handoff_into_other_tenants_session_is_rejected_identical_to_nonexistent_session` — the canonical existence-leak pin that this PR's audit emission MUST NOT break. Green = the audit row writes happen on the substrate side without affecting the public-API observable error.

## Anti-theater check

This is real substrate work, not theater:

- The new SELECT bypasses tenant filtering ON PURPOSE (caller-tenant-FREE lookup) — but it's gated behind the standard tenant-pinned `find_session` returning None first. The cross-tenant SELECT can never run for a caller who legitimately owns the session.
- Genuine-nonexistent path (test 2) proves the helper bails early at `find_session_owner_tenant() → None`, never reaches the audit-write code path.
- Byte-identical-error path (test 3) proves the public API contract is unchanged.
- Payload includes `payload_hash` not `payload` — the attacker's payload contents are not leaked into the victim tenant's audit chain.
- Self-attempt guard (`:412-419`) is a defence-in-depth check that's unreachable today but protected from future refactor damage.
- 4 demote cycles did NOT touch the production code — every cycle was a `git push --force-with-lease` lost or doc-tree-clobber problem; substrate stayed stable.

## Gap / follow-up brief candidates

- **NEW P3 `ops-conductor-fix-request-comment-needs-stale-doc-tree-cleanup-recipe`** (~10-20 LOC docs): tick-131 demoted #2131 4th-time over **doc-tree clobber** (T21-41 audit + evaluator round-44 overlay-recheck + peer codex heartbeats), but no fix-request comment has documented the exact `git checkout origin/main -- <paths>` recipe to recover. Workers re-claim, re-rebase, and re-clobber. A 5-line recipe in the row's fix-request comment would have closed this 24h+ ago.

## File layout for this round

```
docs/skycot/evals/scheduled/2026-05-24/round-45-pr-2131-handoff-cross-tenant-audit-kind-pre-merge-anti-theater-verify.md  (this file)
docs/skycot/evals/scheduled/2026-05-24/round-45-pr-2169-cron-surface-contract-anti-theater-verify.md
docs/skycot/evals/scheduled/2026-05-24/round-45-process-drift-and-p0-ageing-watch.md
```

23rd anti-theater-honest substrate contract on main this week (pending merge).
