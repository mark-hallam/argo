# fix-split-event-tests-rs-loc-budget-breach

**Owner**: claude or codex (fast-mode mechanical-extraction candidate).
**Authored against**: origin/main `452dd1885d22be18cc604a1146cf8bedc9e73fe0` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-split-event-tests-rs-loc-budget-breach`.
**Unblocks**: Track P P-02 (`pitch-truth-boundary-runtime-enforcement`) — currently PRE-MERGE-BLOCKED on PR #2295 because the implementation grows `crates/argo-audit/src/event_tests.rs` from 499 LOC (origin/main, AT ceiling) to 512 LOC, breaching HARD RULE 6 (no file > 500 LOC).

🔴 **P1 ARCHITECTURAL-PREP — splits `event_tests.rs` so the rebased P-02 PR (#2295) can land cleanly.**

Evaluator round-65 (2026-05-25T14:47Z, `docs/skycot/evals/scheduled/2026-05-25/round-65-pr-2295-track-p-p-02-truth-boundary-blocked-conflict-and-loc.md`) called this split out as the prerequisite for unblocking the Track P v1.0 demo headline (operator-locked). Carries from rounds 60, 61, 64.

## Current state

- `crates/argo-audit/src/event_tests.rs` = 499 LOC on `origin/main` (1 below ceiling — zero headroom for new tests).
- `crates/argo-audit/src/event.rs` = 262 LOC; pulls in tests at line 261 via `#[path = "event_tests.rs"] #[cfg(test)] mod tests;`.
- `AuditKind::all().len()` count on main = **73**; after P-02 rebases + adds 3 new variants (`ClaimAllowed` / `ClaimBlocked` / `ClaimEscalated`) + accounting for `WorkerKpiRolled` that landed mid-flight = **76**. P-02 adds ~13 LOC of new tests for the 3 Claim variants → 512 LOC pre-split.

## Scope

1. **Verify current state**: `wc -l crates/argo-audit/src/event_tests.rs` reports 499.
2. **Pick the split axis** (pick whichever yields the cleanest sibling cluster):
   - **Option A (variant-cluster split — RECOMMENDED)**: keep the meta-tests (`label_round_trip_for_every_kind`, `all_kind_count_tracks_declared_variants`) in `event_tests.rs`. Move per-cluster round-trip tests into siblings:
     - `event_tests_workflow_and_worker_duty.rs` (workflow.* + worker_duty.* + worker_kpi.*)
     - `event_tests_approval_and_genesis.rs` (approval.* + genesis.*)
     - `event_tests_cross_tenant.rs` (CrossTenant* variants — the existing pattern this PR will extend)
     Goal: post-split `event_tests.rs` ≤ 350 LOC; new siblings each ≤ 200 LOC.
   - **Option B (per-test-fn-density split)**: extract any tests that exceed 30 LOC into siblings, keep small one-liners in `event_tests.rs`. Lower delta, but less semantic grouping.
   - **Option C (mass extract — one big sibling)**: move everything except the meta-tests to a single `event_tests_kinds.rs`. Simplest mechanical change, but the sibling will itself end up ~400 LOC and the same headroom problem recurs in one rebase.
3. **Implement the chosen split**. All test fns stay in the same crate; siblings are wired the same way `event_tests.rs` is — via `#[path = "..."] #[cfg(test)] mod ...;` declarations in `event.rs`. Or pull all sibling declarations into a single `#[cfg(test)] mod tests;` block inside `event.rs` that imports them via `#[path]` siblings.
4. **Verify all tests still run**: `cargo test -p argo-audit` should report the same passing count post-split (no test additions/removals — pure relocation).
5. **Verify clippy**: `cargo clippy -p argo-audit --all-targets -- -D warnings` clean.
6. **Acceptance**: post-split `event_tests.rs` ≤ 350 LOC (gives P-02 ~150 LOC headroom for the 3 new variant tests + any future audit-kind additions).
7. **Once MERGED**: post a comment on PR #2295 prompting rebase + variant-count bump to 76 (separate `fix-pr-2295-rebase-and-bump-variant-count-to-76` brief, authored once this lands).

## Acceptance criteria

- [ ] `wc -l crates/argo-audit/src/event_tests.rs` ≤ 350 LOC post-split.
- [ ] All sibling files ≤ 200 LOC each.
- [ ] `cargo test -p argo-audit` reports same pass count as pre-split.
- [ ] `cargo clippy -p argo-audit --all-targets -- -D warnings` clean.
- [ ] No public-surface change (this is a test-file refactor; nothing leaves the crate).
- [ ] PR body documents chosen Option (A / B / C) + LOC delta per file + total test-fn count pre/post (must match).

## May-touch

- `crates/argo-audit/src/event_tests.rs` (split)
- `crates/argo-audit/src/event_tests_*.rs` (NEW sibling files per chosen option)
- `crates/argo-audit/src/event.rs` (narrow — add `#[path]` mod declarations for new siblings)

## Out of scope

- Adding new `AuditKind` variants (P-02's job once this lands)
- Renaming or restructuring `AuditKind` itself
- Touching `event_kind.rs` / `event_kind_docs.rs` / `sink.rs` / `csv.rs` / other audit modules
- Modifying any test logic — pure relocation only

## Effort

~50-200 LOC delta + sibling file creation. ~30-45 min mechanical work. Fast-mode candidate (no architectural judgement required — Option A is the recommended axis).
