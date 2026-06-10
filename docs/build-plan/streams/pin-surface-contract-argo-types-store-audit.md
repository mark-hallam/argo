# pin-surface-contract-argo-types-store-audit

Authored against: origin/main ca46b97f1e42d9d093d275763b55a04add3d65c3 on 2026-06-10
**Validate before claim**: bash scripts/check-brief-freshness.sh pin-surface-contract-argo-types-store-audit

## Goal

Add `tests/surface_contract.rs` public-surface pins to the three highest-stakes
crates that lack them, so a silent public-API change in the canonical-types
home, the database layer, or the audit crate becomes a loud test failure.

## Why now (test-health audit 2026-06-10, F4)

14 crates have `crates/*/tests/surface_contract.rs`, but the crates where a
silent surface change hurts MOST are unpinned (verified 2026-06-10 —
`ls crates/<c>/tests/surface_contract.rs` absent for all three):
- **argo-types** — the canonical-types home (the whole point of the AuditRow /
  SessionId / Role canonicalization); an unpinned re-export drift here is the
  worst case.
- **argo-store** — the database heart (17 integration test files but no surface
  lock).
- **argo-audit** — the hash-chained audit writer (differentiator #1).

## Fix

For each crate, add `tests/surface_contract.rs` following the EXISTING template
(copy the shape from e.g. `crates/argo-eval/tests/surface_contract.rs` or
`crates/argo-acp/tests/surface_contract.rs`): import the crate's public items
and assert their existence/shape (constructors, key enum variants, fn
signatures via type-level asserts). Pin the LOAD-BEARING public surface, not
every item — focus on the canonical types + the writer/store entry points.

## May touch

- `crates/argo-types/tests/surface_contract.rs` (NEW)
- `crates/argo-store/tests/surface_contract.rs` (NEW)
- `crates/argo-audit/tests/surface_contract.rs` (NEW)
- the three `Cargo.toml`s only if a dev-dep is needed for the pin

## Acceptance / verifier

1. `cargo test -p argo-types -p argo-store -p argo-audit --test surface_contract`
   compiles + passes.
2. A deliberate rename of one pinned public item makes the pin FAIL to compile
   (demonstrate in the PR body, then revert).

## Size / owner

~3 × 40-120 LOC pin files. Owner: claude. 🟡 P2 — closes the gap where the
highest-stakes crates are unprotected.
