# Mission load-bearing wire — per-delegation budget tree

**Owner**: claude. **Depends on**: existing `subtask` table (verify slot in first 5min via `ls crates/argo-store/migrations | grep subtask`); existing budget primitives (verify: `grep -rn 'Budget' crates/argo-runtime/src | head`); `record_decomposition` writes subtask rows (per `crates/argo-runtime/src/planner.rs:48`).
**Authored against**: origin/main `e44f6e8a` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-delegation-budget-tree`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 2 #57 — "Add per-delegation budget tree." Today a delegating worker's budget is opaque to its children — each child runs its own budget from scratch, which means total spend is unbounded for a deep tree. Mission promise: when worker A delegates a subtask to worker B with budget X, B's effective remaining budget is `min(B_own, X - B_consumed_by_siblings)`. Recursive sums roll up the tree at audit time.

## Scope

1. **NEW migration**: `<next-slot>__subtask_budget.sql`:
   ```sql
   ALTER TABLE subtask ADD COLUMN budget_usd_cents INTEGER NULL;
   ALTER TABLE subtask ADD COLUMN spent_usd_cents INTEGER NOT NULL DEFAULT 0;
   ```
   Versioned counterpart in `crates/argo-store/src/migrations/v<next>.rs`. Bump `EXPECTED_SCHEMA_VERSION`. Verify next free slot vs in-flight briefs.

2. **`SubtaskStore` API additions** (sibling module if subtask.rs over LOC):
   - `set_subtask_budget(subtask_id, budget_usd_cents)` — called by parent when delegating.
   - `record_subtask_spend(subtask_id, delta_usd_cents)` — called as the child accumulates.
   - `get_remaining(subtask_id) -> Option<i64>` — returns `Some(budget - spent)` if budget set; `None` for unbounded.
   - `walk_tree(root_subtask_id) -> Vec<(SubtaskRow, i64 /* aggregated_spent */)>` — recursive roll-up using a CTE.

3. **`record_decomposition` extension**: accept an optional `parent_budget_usd_cents` parameter. If parent had a budget, divide it across children using an even split as v1 default (smarter allocators are follow-up). Each child's row gets `budget_usd_cents = parent / N`.

4. **Runtime enforcement**: when a worker turn finishes, compute incremental usage cost (existing token-charge primitive — verify location). Call `record_subtask_spend` with the delta. Before the NEXT turn starts, check `get_remaining` — if it's `Some(x)` and `x <= 0`, abort the subtask with `BudgetExceeded` error + audit row.

5. **NEW CLI**: `argo subtask budget <root_id> [--json]` — uses `walk_tree`. Output a tree:
   ```
   root (id=…) budget=$10.00 spent=$3.21 remaining=$6.79
   ├── child-A (id=…) budget=$5.00 spent=$2.10 remaining=$2.90
   └── child-B (id=…) budget=$5.00 spent=$1.11 remaining=$3.89
   ```

6. **Empty/legacy case**: subtasks predating the migration have NULL budget → `get_remaining` returns `None`, runtime treats as unbounded (no enforcement). Don't crash on NULL.

This is NOT in scope: dynamic re-allocation between siblings; tenant-level budget pools (separate brief); UI/dashboard rendering (CLI only); auto-pause on near-exhaustion (separate alert brief — `codex-budget-alert-channel` exists for tenant-level).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/migrations/<next-slot>__subtask_budget.sql` (NEW) — verify slot.
- `crates/argo-store/src/migrations/v<next>.rs` (NEW) — versioned migration; bump EXPECTED_SCHEMA_VERSION at `crates/argo-store/src/lib.rs:151`.
- `crates/argo-store/src/subtask.rs` OR sibling `subtask_budget.rs` (NEW, ≤ 250 LOC) — the 4 new methods + CTE helper. Verify subtask.rs LOC; spill to sibling if it'd breach 500.
- `crates/argo-store/src/subtask_budget_tests.rs` (NEW, ≤ 200 LOC) — 8 unit tests: set-budget-roundtrip; record-spend-accumulates; get-remaining-with-budget; get-remaining-null-budget; walk-tree-2-level; walk-tree-3-level-aggregates; spend-exceeds-budget-still-records; set-budget-on-legacy-null-row.
- `crates/argo-runtime/src/planner.rs` — narrowly scoped: extend `record_decomposition` signature with `parent_budget: Option<i64>`. Default split = even. ≤ 25 LOC delta. **Verify file stays < 500 LOC** (currently 286, safe).
- `crates/argo-agent-runner/src/turn.rs` OR sibling — narrowly scoped: post-turn hook calling `record_subtask_spend(delta)` + pre-turn `get_remaining` check. ≤ 40 LOC delta. Spill to `turn_budget.rs` (NEW, ≤ 150 LOC) if needed.
- `crates/argo-agent-runner/src/turn_budget_tests.rs` (NEW, sibling) — 4 unit tests.
- `crates/argo-cli/src/handlers/subtask_budget.rs` (NEW, ≤ 200 LOC) — the new CLI handler.
- `crates/argo-cli/src/cli_subtask_cmds.rs` (verify path) OR `cli_command_enum.rs` — narrowly scoped: ADD `Budget` subcommand variant.

## Files you must NOT touch

- `crates/argo-store/src/subtask.rs` existing methods — extend with new sibling functions only, don't touch existing
- Any tenant-budget code (`codex-budget-*` briefs own that surface)
- Any non-store/non-runtime/non-agent-runner/non-cli crate

## Acceptance criteria

- [ ] Migration adds `budget_usd_cents` + `spent_usd_cents` columns to subtask
- [ ] EXPECTED_SCHEMA_VERSION bumped
- [ ] 4 new SubtaskStore methods exist
- [ ] `record_decomposition` accepts optional parent budget + does even split
- [ ] Runtime calls `record_subtask_spend` per turn; aborts on remaining ≤ 0
- [ ] `argo subtask budget <id>` renders tree with --json envelope
- [ ] Legacy NULL-budget rows treated as unbounded (no crash, no enforcement)
- [ ] 8 store unit tests + 4 runtime unit tests + 1 integration smoke (e2e CLI command)
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-store -p argo-runtime -p argo-agent-runner -p argo-cli` clean
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-delegation-budget-tree`
- PR title: `delegation: per-subtask budget tree with roll-up + abort-on-exceed (mission-delegation-budget-tree)`

## Notes for the implementer

- Migration slot: verify against in-flight briefs at claim time. Other briefs touching migrations: search `grep -l 'NN__' docs/build-plan/streams/*.md` for collisions.
- The "even split across children" is a deliberate v1 simplification — a smarter allocator (weight by est. complexity) is a separate brief.
- The CTE for `walk_tree`: SQLite supports recursive CTEs (`WITH RECURSIVE`). Use `parent_subtask_id` if that column exists; if not, add it in the same migration (verify schema first).
- For the per-turn spend delta: token-cost is already computed somewhere — find it (`grep -rn 'tokens_used\|usd_cents' crates/argo-agent-runner/src/`) and call our new method right where the existing accounting happens.
- BLOCK and request revision rather than touch existing budget primitives.
