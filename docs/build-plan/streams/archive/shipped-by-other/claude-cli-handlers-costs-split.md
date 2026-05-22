# claude-cli-handlers-costs-split — split costs.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Phase A architectural prep — defensive split.**

**Authored against**: origin/main d564971d on 2026-05-09 (conductor-authored after observing costs.rs at 498/500 LOC).

## Why

`crates/argo-cli/src/handlers/costs.rs` is at 498 LOC on current main — 2 LOC of headroom against the 500-LOC HARD RULE 6 ceiling. Phase B cost-tracking briefs (forecast / by-day / by-skill / per-tenant) all touch this. Preemptive split prevents future ceiling-refusals.

Pure refactor; mirrors proven pattern (#932, #956, #1087, #1088, #1097, #1102, etc.).

## Approach (concrete)

1. Read `handlers/costs.rs` end-to-end. Identify a self-contained section (likely per-subcommand handlers or formatters).
2. Move to `crates/argo-cli/src/handlers/costs_<topic>.rs`.
3. Re-export via `pub(crate) use`.

Target: `costs.rs` ≤ 350 LOC.

## Files you may touch

- `crates/argo-cli/src/handlers/costs.rs` — narrowly: extract sibling, re-export
- `crates/argo-cli/src/handlers/costs_<topic>.rs` (NEW, ≤ 250 LOC)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 `pub mod` line if needed

## Files you must NOT touch

- Other costs-related handlers (`costs_by_day.rs`, `costs_forecast.rs`) — separate concerns
- Any test file other than to verify no behaviour change

## Acceptance criteria

- [ ] `costs.rs` ≤ 350 LOC (≥ 150 LOC headroom)
- [ ] Sibling ≤ 250 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` clean
- [ ] No public API changes

## Branch + PR

- Branch: `claude-cli-handlers-costs-split`
- PR title: `cli: split handlers/costs.rs below 500-LOC ceiling (claude-cli-handlers-costs-split)`

## Unblocks

Future cost-tracking briefs that extend costs.rs.
