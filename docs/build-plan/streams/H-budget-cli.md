# Stream H (follow-up) — Worker budget CLI subcommands

**Owner**: codex. **Depends on**: H-spend-persistence merged.

## Scope

Operator-facing CLI for inspecting per-worker budget state.
Mirrors `argo costs recent` from existing work.

Subcommands:
- `argo budget show <worker_id>` → daily + monthly rollup,
  remaining headroom against the policy caps, last 5 charges.
- `argo budget recent [--limit N] [--worker <id>] [--since
  <iso8601>]` → table or JSON of recent charges.

Universal `--json` per existing CLI convention.

## Files you may touch
- `crates/argo-cli/src/cli_budget_cmds.rs` (NEW)
- `crates/argo-cli/src/handlers/budget.rs` (NEW)
- `crates/argo-cli/src/cli.rs` — narrowly scoped: register
  the `Budget(...)` arm. ONE arm, no other edits.
- `crates/argo-cli/src/main.rs` — narrowly scoped: dispatch
  the new arm. ~5 lines.
- `crates/argo-cli/src/handlers/mod.rs` — ONE line `pub mod
  budget;`.

## Files you must NOT touch
- `crates/argo-identity/src/worker_budget.rs` — reuses the
  H-spend-persistence helpers; don't extend.
- `crates/argo-identity/src/auto_approval.rs` — read only.

## Acceptance criteria
- [ ] `argo budget show <worker_id>` queries spend_today_for_
      worker + a new spend_this_month query (codex adds the
      query helper if missing — narrowly scoped to
      `worker_budget.rs`).
- [ ] `--json` flag on every subcommand.
- [ ] `argo budget recent --since 2026-05-01T00:00:00Z`
      filters by ISO-8601 timestamp.
- [ ] Headroom calculation: if policy cap is set, output
      `remaining_today_usd` + `remaining_month_usd`. If unset,
      output `null` for those fields.
- [ ] Integration test using assert_cmd + an in-memory store
      with seeded charges.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Every authored file < 500 LOC

## Branch + PR
- Branch: `codex-H-budget-cli`
- PR title: `cli: argo budget show + recent subcommands (H-budget-cli)`

## Notes
- USD micros → display USD by `/ 1_000_000` and format with 2
  decimals; clamp to 4 if it underflows in display.
- Mirror the columns + ordering of `argo costs recent` so the
  operator's mental model is consistent.
