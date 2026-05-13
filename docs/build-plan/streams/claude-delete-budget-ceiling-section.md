# claude-delete-budget-ceiling-section — delete orphan `BudgetCeilingSection` parallel config (85 LOC + tests)

**Owner**: claude. **Depends on**: nothing (pure delete).
**Authored against**: origin/main `bf8236e3` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-delete-budget-ceiling-section`.

> **PURE DELETE 2026-05-14 by conductor (CEO-authorized Phase 1 cleanup).** Audit cluster 5 confirmed `argo_config::budget_ceiling::BudgetCeilingSection` (85 LOC + tests) is **orphan** — public API with zero non-test callers. Parallel to `argo_config::sections_budget::BudgetConfig` which IS actively used.

## Scope

Delete `crates/argo-config/src/budget_ceiling.rs` + `budget_ceiling_tests.rs`. Remove the module declaration from `lib.rs` and any `pub use` re-exports.

The intended wire-up was `mission-budget-gate-route-callers` (referenced in `budget_ceiling.rs:9`); that brief was never authored or landed. If future operator demand surfaces hard `[budget.ceiling]` enforcement, that's a future brief — built against a fresh design that may differ from this orphan's shape.

**Not in scope**: changing `BudgetConfig` (the working one in `sections_budget`); deleting `argo-store::spend_ceiling` (the runtime enforcement path); removing the `[budget.ceiling]` TOML key from manifests if any.

## Files you may touch

- `crates/argo-config/src/budget_ceiling.rs` — `git rm`.
- `crates/argo-config/src/budget_ceiling_tests.rs` — `git rm`.
- `crates/argo-config/src/lib.rs` — narrowly: remove `pub mod budget_ceiling;` + any `pub use`. ≤5 modified lines.
- Any caller of `argo_config::budget_ceiling::*` that the audit might have missed (`rg 'budget_ceiling' crates/`). Surface in PR body if found.

## Files you must NOT touch

- `crates/argo-config/src/sections_budget.rs` — locked; this is the actively-used surface.
- `crates/argo-store/src/spend_ceiling.rs` — different concept (read-side enforcement); locked.
- Any TOML manifest with `[budget.ceiling]` keys — leave them; the runtime read path uses `argo-store::spend_ceiling`.

## Acceptance criteria

- [ ] Both files removed.
- [ ] `lib.rs` cleaned of references.
- [ ] `cargo build -p argo-config --all-targets` succeeds.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-config` passes.
- [ ] `rg 'BudgetCeilingSection\|budget_ceiling::' crates/` returns no production hits after delete.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-delete-budget-ceiling-section`
- PR title: `cleanup: delete orphan BudgetCeilingSection config (85 LOC) (claude-delete-budget-ceiling-section)`

## Notes for the implementer

- ~30 min worker time. Pure delete.
- Smaller cousin of the argo-gateway delete; same shape (orphan that the codebase routed around).
