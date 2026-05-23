# Round 40 — PR #2163 anti-theater verify: AuditKind::ApprovalDeclined variant + Claim-6 row-4 pin test

- **Date**: 2026-05-23 (real UTC; Melbourne local 2026-05-24)
- **Round**: 40 (scheduled-task hourly fire, read-only)
- **Pillar / Claim**: Claim 6 — Multi-tenant by construction (mission-dashboard row 4: approval audit kinds)
- **Verdict**: 🟢🟢 **PASS** (substrate honest, anti-theater clear, mechanical brief deviation justified)

## Setup

- PR: `mission — argo-audit — add AuditKind::ApprovalDeclined variant + row-4 pin test (#2163)`
- Merged at: 2026-05-23T17:11:42Z (commit `952e42591`)
- Files touched: `event_kind.rs` (+19/-0), `event.rs` (+3/-0), `event_tests.rs` (+14/-1), `tests/claim_6_approval_audit_kinds.rs` (+80 new file). Total: +116/-1.
- Cold-built and ran the new pin test on origin/main HEAD `9f67ca418`:
  ```bash
  cargo test -p argo-audit --test claim_6_approval_audit_kinds
  ```

## Observed

```
running 3 tests
test dashboard_row_4_kinds_round_trip ... ok
test dashboard_row_4_kinds_listed_exactly_once ... ok
test dashboard_row_4_kinds_serde_shape_is_snake_case ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Production wiring spot-checks:

- `crates/argo-audit/src/event_kind.rs:99` — `ApprovalDeclined,` variant added to the `AuditKind` enum.
- `crates/argo-audit/src/event.rs:33` — `Self::ApprovalDeclined => "approval.declined"` in `label()` (matches family convention: `approval.granted` / `approval.revoked` / `approval.replied`).
- `crates/argo-audit/src/event.rs:112` — `"approval.declined" => Self::ApprovalDeclined` in `from_label()`.
- `crates/argo-audit/src/event.rs:191` — `Self::ApprovalDeclined,` listed in `all()` (the iterator the chain-verifier walks).
- `crates/argo-audit/src/event_tests.rs:15` — `assert_eq!(AuditKind::all().len(), 70);` — matches the PR-body claim "Existing `event_tests.rs::all_kind_count_tracks_declared_variants` bumps from 69 → 70".

Three new pin tests (lines 51-80 of the new test file):

1. `dashboard_row_4_kinds_round_trip` — for all four kinds named in the dashboard cell (`ApprovalGranted` / `ApprovalDeclined` / `ApprovalReplied` / `ApprovalRevoked`), `label()` returns the family-convention string and `from_label(label())` round-trips back to the same variant.
2. `dashboard_row_4_kinds_listed_exactly_once` — each kind appears exactly once in `AuditKind::all()` (no silent drop, no duplication).
3. `dashboard_row_4_kinds_serde_shape_is_snake_case` — each kind serialises to its stable snake_case string (`approval_granted` / `approval_declined` / …), guarding the on-disk `kind` column shape.

## Anti-theater check

This is a TYPE-LEVEL contract pin, not a behavioural pin — the test does not assert anything about what production code writes. That is **honest** because:

1. **The PR body explicitly says emission is deferred.** Reproduced verbatim: *"Production emission of `ApprovalDeclined` is deferred to a follow-up wire-up brief."* The test file's module docstring (lines 31-40) repeats this: emission belongs in the composition layer (gateway `routes_approvals_json::decide_response` is the leading candidate), not in `argo-store::approvals::decide_request`.
2. **The dependency-cycle argument is structurally checkable.** `argo-audit/Cargo.toml` already depends on `argo-store`; a reverse `argo-store → argo-audit` edge would form a cycle. So Option A's "add the variant + pin its contract; defer wire-up" is the only structurally feasible shape today.
3. **Zero production callers of `decide_request` confirmed via `rg`** — all 5 hits are `*_tests.rs` files inside `argo-store/src/`. The PR body's claim is verified by direct grep.
4. **The pin tests are not tautological.** They assert real behaviour (`label()`, `from_label()`, `all()`, serde) against three independent oracles. If a future refactor renames the variant or drops it from `all()`, every test fails.
5. **The count-bump test (`all_kind_count_tracks_declared_variants` = 70)** is a brittle-on-purpose guard: any new audit-kind variant added without a corresponding test bump fails immediately, forcing the author to acknowledge the new kind in the test suite. This is the same brittle-guard pattern used elsewhere in the audit chain.

## Gap / follow-up brief candidates

- **`mission-claim-6-row-4-approval-audit-kinds-wire-emission`** (P1, ~80-150 LOC) — the follow-up wire-up brief named in #2163's body. Composition site: most likely `crates/argo-cli/src/handlers/gateway/routes_approvals_json::decide_response`, which already calls `worker_actions_transitions::approve()` / `cancel()`. Add an `AuditSink::write_with_tenant` call alongside the cancel path so a real `approval.declined` row is emitted on every operator decline. Pin test: assert the row appears exactly once per decline, payload carries `tenant_id` + `request_id` + `decided_by`, and the hash chain stays clean. Anti-theater requirement: the pin must use `LIVE_NOT_FINDINGS_ONLY` mode (assert presence, not absence) — the round-37 #2146 + round-37 #2144 pattern of un-ignoring a FINDINGS-ONLY pin once the production seam wires up is the right shape.

This brief was already named-in-flight by the row-4 pin's docstring (lines 31-40 of the new test file). Re-surface for conductor visibility — not new, just keep on the queue.

## Headline number

**25th anti-theater-honest substrate contract on main this week.** (Tally from round-39: 24th.)

Claim 6 MVP coverage chain: the substrate variant is now real; the row-4 dashboard cell will turn 🟢🟢 once the wire-up brief lands. Currently the row sits at 🟢 substrate-only.
