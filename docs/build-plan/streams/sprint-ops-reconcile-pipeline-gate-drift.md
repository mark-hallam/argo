# sprint-ops-reconcile-pipeline-gate-drift

**Owner**: claude or codex (mechanical).
**Depends on**: none. **Un-blocks clean pre-merge gating for the whole sprint pipeline.**
**Authored against**: origin/main `a0204a379` on `2026-06-04`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh sprint-ops-reconcile-pipeline-gate-drift`.

## Why

During conductor verification of the sprint PRs (#2675, #2676), running
`scripts/conductor-pre-merge-gate.sh --skip-clippy` surfaced that the FULL gate is
currently RED on `main` due to **pre-existing drift unrelated to any one PR** — so every
sprint PR trips the same two gates and the conductor is admin-merging verified PRs with
boundary notes as a workaround. Clearing this restores clean gating.

- **Gate 5b (routes-catalog completeness):** 8 `/api/v1/*` families are dispatcher-registered
  but absent from `crates/argo-cli/src/handlers/gateway/routes_catalog.rs`.
- **Gate 3 (LOC budget, hard ceiling 500):** 3 files over the ceiling.

## What

### (a) Reconcile `routes_catalog.rs` — add the 8 missing families
Add catalog entries (matching the existing entry format) for, with their dispatched methods:

| family | methods |
|---|---|
| `api-keys` | DELETE, POST |
| `governance` | GET |
| `gtm` | POST |
| `marketing` | POST |
| `pitch` | POST |
| `providers` | (verify methods at claim time via the dispatcher) |
| `voice` | GET, POST |
| `waitlist` | POST |

Re-verify each family's actual dispatched methods + path at claim time
(`rg '"/api/v1/<family>' crates/argo-cli/src/handlers/gateway/`). Then
`bash scripts/check-routes-catalog-complete.sh` → must report 0 missing. If adding the
entries would push `routes_catalog.rs` over 500 LOC, extract the list into a sibling module.

### (b) Split the 3 over-ceiling files (mechanical sibling-module extraction)
- `crates/argo-cli/src/handlers/gateway/routes_retrain_tests.rs` (627)
- `crates/argo-cli/src/handlers/gateway/routes_genesis_stream_tests.rs` (832)
- `crates/argo-cli/src/handlers/gateway/routes_retrain_helpers.rs` (529)

Extract cohesive groups into sibling `*_<topic>.rs` modules (established pattern, e.g.
`store_helpers.rs`), preserving every test + `pub(super)` call site via `mod`/`use`. No
behavior change. Each resulting file < 500 LOC.

## May-touch
- `routes_catalog.rs` (+ a sibling module if it would exceed 500 LOC)
- the 3 files above + their new sibling modules + parent `mod.rs` declarations
- `docs/build-plan/BUILD_STATUS.md` (ledger flip on PR open)

Do NOT change any route behavior, handler logic, or test assertions — pure catalog-data
addition + mechanical file extraction.

## Acceptance criteria
- [ ] `bash scripts/check-routes-catalog-complete.sh` → 0 missing families.
- [ ] No file in `crates/argo-cli/src/handlers/gateway/` over 500 LOC.
- [ ] `bash scripts/conductor-pre-merge-gate.sh --skip-clippy` → gates 3 and 5b both PASS on clean main.
- [ ] `cargo check -p argo-cli --tests` GREEN; touched-test runs GREEN.
- [ ] No fn > 80 LOC, no `unwrap`/`expect` outside `#[cfg(test)]` introduced.

## Out of scope
- The pre-existing `await_holding_lock` clippy errors in `routes_settings_credentials_tests.rs`
  (separate; `--skip-clippy` covers them; do NOT bundle).

## Parallel-surface check
- `rg 'fn .*catalog' crates/argo-cli/src/handlers/gateway/` — extend the single canonical catalog, do not add a parallel one.

## Source
Found 2026-06-04 during conductor verification of #2675/#2676. Audit: `docs/skycot/audit/2026-06-04-day0-sprint-gap-audit.md`.
