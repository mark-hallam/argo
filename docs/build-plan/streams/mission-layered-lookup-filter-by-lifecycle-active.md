# mission-layered-lookup-filter-by-lifecycle-active

**Authored against:** `origin/main` 6d148f0e1585b73b9527ef3ed8a0201d27eb4aca on 2026-05-23.
**Validate before claim:** `bash scripts/check-brief-freshness.sh mission-layered-lookup-filter-by-lifecycle-active`.

## Why this brief exists

Surfaced as the FINDINGS pin from PR #2122 (`eval-skill-rejection-flow`, MERGED 2026-05-23T10:50Z). Two `#[ignore]`'d tests in `crates/argo-skill/tests/skill_rejection_flow.rs` currently pin a contract gap:

> `LayeredLookup::list_for_action` and `LayeredLookup::find` route through `list_active` / `find_active_by_name`, which filter only on `superseded_by IS NULL`. A `Rejected` skill row has `lifecycle_state = Rejected` AND `superseded_by IS NULL`, so it survives the filter and leaks into the worker prompt.

Result: when the judge **rejects** a candidate skill, the rejection is durably recorded — but `LayeredLookup` (the surface workers consume) keeps returning the rejected row until a successor row is inserted.

## Acceptance criteria

1. Update `crates/argo-skill/src/store_lookups.rs` (lines ~43 and ~123 per the PR #2122 FINDINGS note) so that `list_active` / `find_active_by_name` additionally filter `lifecycle_state = 'active'` (or equivalent — confirm the exact enum/string at claim time).
2. Un-`#[ignore]` the two FINDINGS-ONLY tests in `crates/argo-skill/tests/skill_rejection_flow.rs` and confirm they pass.
3. Verify `LayeredLookup::list_for_action` / `::find` callers across the worker prompt path (e.g. `crates/argo-agent-runner/src/turn_helpers.rs`) still see the expected `active` rows in the happy path. No new flakes in `cargo test -p argo-skill -p argo-agent-runner`.
4. Per-crate `cargo clippy -p argo-skill -- -D warnings` clean.

## Parallel-surface declarations

- Extends existing `LayeredLookup::list_active` / `find_active_by_name` in `crates/argo-skill/src/store_lookups.rs`. No new pub surface.
- Wire-up consumer: already wired — workers consume `LayeredLookup` at `crates/argo-agent-runner/src/turn_helpers.rs:65` (per claim-4 SHIPPED memory). No new wire needed.

## May-touch

- `crates/argo-skill/src/store_lookups.rs`
- `crates/argo-skill/tests/skill_rejection_flow.rs` (un-`#[ignore]`)

## Boundary

- Do NOT touch `LayeredLookup::list_pending_review` / similar discovery surfaces — they should still see all lifecycle states.
- The pre-existing `store_worker_overlay_tests` flakes (`list_for_worker_filters_other_workers` + `list_for_worker_returns_attached_active_overlays`) reproduce on clean `origin/main`; outside may-touch.

## Sizing

~30-80 LOC (one filter clause + 2 un-`#[ignore]`s + verify). Fast-mode candidate.

**🟡 P2 CLAIM-4 (skill loop hygiene)** — claim-4 worker-load path leaks rejected rows; once this brief lands the eval-skill-rejection-flow contract is fully green (no `#[ignore]`s).
