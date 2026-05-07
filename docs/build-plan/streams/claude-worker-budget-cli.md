# `argo worker budget set` — write per-worker spend caps

**Owner**: claude. **Depends on**: —

## Why

`argo-store::worker_budget` already exists — the spend column is queried
on every dispatch and gates rate. But the only path to *populate* the
cap rows today is direct SQL or test seeds. Operators need a CLI:
`argo worker budget set <worker> --daily $5 --monthly $100`. The
read side already exists; this is the missing write.

## Scope

- `argo worker budget set <id> [--daily <usd>] [--monthly <usd>] [--clear]`
- `argo worker budget show <id>` — render current caps + period-to-date spend.
- USD parsed as decimal dollars, stored as integer micros (existing convention).
- Negative values rejected; 0 is allowed (means "deny all").
- `--clear` removes the cap row entirely (back to tenant default).
- All writes are idempotent: setting the same value twice is a no-op
  to the user.

## Approach (concrete)

1. **NEW or extend** `crates/argo-store/src/worker_budget_write.rs`
   (~100 LOC) IF a write helper doesn't already exist (verify via
   `grep -n "INSERT INTO worker_budget\|UPDATE worker_budget" crates/argo-store/src/`):
   - `pub async fn set_cap(store, id, daily_micros, monthly_micros) -> ArgoResult<()>`
   - `pub async fn clear_cap(store, id) -> ArgoResult<()>`
   - 6 unit tests in same file.
2. **NEW** `crates/argo-cli/src/handlers/worker_budget.rs` (~180 LOC)
   IF the file doesn't already exist (one called `budget.rs` exists for
   tenant-level — check the namespace before colliding):
   - `parse_usd(s: &str) -> ArgoResult<i64>` — "5" → 5_000_000 micros, "5.50" → 5_500_000
   - `set` / `show` handlers with markdown + JSON output
3. **NEW** `crates/argo-cli/src/handlers/worker_budget_tests.rs` (~200 LOC):
   - parse_usd: integers, decimals, trailing zero, scientific (reject), negative (reject)
   - set: happy, both zero, clear-after-set
   - show: with cap, without cap, with non-zero spend, with zero spend
4. **Modify** `crates/argo-cli/src/cli.rs` — 1 sub-enum
   `WorkerBudget { Set { id, daily, monthly, clear }, Show { id, json } }`.
   EXHAUSTIVE-MATCH note: handlers/mod.rs dispatch.

## Files you may touch

- `crates/argo-store/src/worker_budget_write.rs` (NEW or extend, ≤ 180 LOC)
- `crates/argo-store/src/lib.rs` (1 pub mod line if NEW)
- `crates/argo-cli/src/handlers/worker_budget.rs` (NEW, ≤ 220 LOC) — name conflict-check first
- `crates/argo-cli/src/handlers/worker_budget_tests.rs` (NEW, ≤ 250 LOC)
- `crates/argo-cli/src/handlers/mod.rs` (1 pub mod + dispatch arm)
- `crates/argo-cli/src/cli.rs` (BLANKET CLI ALLOWLIST — 1 sub-enum)
- `crates/argo-cli/tests/worker_budget_smoke.rs` (NEW, ≤ 150 LOC)

## Files you must NOT touch

- `crates/argo-store/migrations/` — `worker_budget` table already exists
- The check-side path (anomaly detection, dispatch gating) — read-side
  reuses the same column you're writing
- Tenant-level `argo budget` commands — different scope
- `argo-orchestrator/` — out of scope

## Acceptance criteria

- [ ] `argo worker budget set <id> --daily 5 --monthly 100` writes 5_000_000 + 100_000_000 micros
- [ ] `argo worker budget set <id> --daily 5.50` parses to 5_500_000 micros
- [ ] `argo worker budget set <id> --daily 0` allowed; means "deny all"
- [ ] `argo worker budget set <id> --daily -1` rejected
- [ ] `argo worker budget set <id> --clear` removes the cap row
- [ ] `argo worker budget show <id>` renders cap + period-to-date spend
- [ ] Setting same value twice is idempotent
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-store -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-worker-budget-cli`
- PR title: `worker: write-side `argo worker budget set/show` (claude-worker-budget-cli)`

## Notes for the implementer

- BLANKET CLI ALLOWLIST applies.
- The "micros" convention is already in the codebase — search for
  `usd_micros` to match style. Don't invent a new unit.
- File-name conflict check: there's an existing `budget.rs` for
  tenant-level. Your new file is `worker_budget.rs` (singular,
  distinct). If a `worker_budget.rs` already exists in handlers/,
  STOP and either extend it narrowly or surface to conductor.
- Use `rust_decimal` if it's already a workspace dep; otherwise
  hand-roll the dollar parser — DO NOT add a new dep.
