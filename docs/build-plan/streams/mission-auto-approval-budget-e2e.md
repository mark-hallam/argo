# mission-auto-approval-budget-e2e — auto-approval budget + window end-to-end test

**Owner**: claude. **Depends on**: nothing (auto-approval primitives + spend ledger exist). **Tier M — Mission MVP critical path.**

**Authored against**: origin/main 7785708b on 2026-05-10.

## Why

Mission claim: operators can set budget caps and time windows, and actions inside the
envelope auto-execute while actions outside fall to Ask. The primitives exist
(`worker_actions_auto_approval.rs`, spend ledger, time-window gate), but no test
proves the wiring end-to-end. This brief is a single integration test that exercises
the full envelope and asserts spend reconciliation. Cheap insurance against silent
regressions in a load-bearing operator-facing feature.

## Approach (concrete)

1. **NEW** `crates/argo-cli/tests/auto_approval_budget_e2e.rs` (~280 LOC):
   - Set up isolated tmpdir as `ARGO_HOME`.
   - Write fixture `auto_approval.toml`:
     ```toml
     [budget]
     per_action_usd = 5.00
     per_day_usd = 50.00

     [window]
     timezone = "America/New_York"
     weekdays = ["mon","tue","wed","thu","fri"]
     hours = "09:00-17:00"
     ```
   - Inject test clock at `Wed 2026-05-13 14:00 America/New_York` (inside window).
   - Fire 3 candidate actions:
     - Action 1: `cost_usd = 3.00` → assert auto-execute.
     - Action 2: `cost_usd = 4.00` → assert auto-execute (cumulative spend = $7).
     - Action 3: `cost_usd = 6.00` → assert falls to Ask (over per-action cap).
   - Inject test clock at `Sat 2026-05-16 11:00` (outside weekday window).
   - Fire 1 candidate action: `cost_usd = 2.00` → assert falls to Ask (outside hours).
   - Assert spend ledger reconciles: total recorded = $7 (Action 1 + 2; Asks do not
     consume budget).
2. **NEW fixture files** under `crates/argo-cli/tests/fixtures/auto_approval/`:
   - `auto_approval_budget.toml` (the config above)
   - `actions_*.json` for the 4 candidate actions if needed
3. **No new feature code** — all primitives exist. If the test surfaces a gap (e.g. a
   missing test-clock injection helper), add the minimum wiring (≤ 30 LOC) to expose
   it; if more is needed, BLOCK with a clear note.
4. **Rustdoc**: any new `pub` test-helper carries doc + example.

## Files you may touch

- `crates/argo-cli/tests/auto_approval_budget_e2e.rs` (NEW, ≤ 320 LOC)
- `crates/argo-cli/tests/fixtures/auto_approval/auto_approval_budget.toml` (NEW)
- `crates/argo-cli/tests/fixtures/auto_approval/*.json` (NEW, as needed)
- ≤ 30 LOC of test-helper wiring in `crates/argo-cli/src/handlers/worker_actions_auto_approval.rs`
  ONLY if the test surfaces a missing test-injection seam. Prefer to call existing
  entrypoints first; only add a seam if absolutely necessary.

## Files you must NOT touch

- Auto-approval core logic — call existing entrypoints.
- Spend ledger schema or core logic — call existing entrypoints.
- Time-window gate core logic — call existing entrypoints.
- Any state-machine code beyond observing transitions.

## Acceptance criteria

- [ ] Test fires 3 in-window actions; 2 auto-execute, 1 falls to Ask (over per-action cap).
- [ ] Test fires 1 out-of-window action; falls to Ask.
- [ ] Spend ledger total reconciles to $7.00 (only auto-executed actions consume budget).
- [ ] Test is deterministic — uses injected clock, no real-time dependency.
- [ ] `cargo test -p argo-cli --test auto_approval_budget_e2e` clean.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.

## Branch + PR

- Branch: `claude-mission-auto-approval-budget-e2e`
- PR title: `tests: end-to-end auto-approval budget + window envelope (mission-auto-approval-budget-e2e)`
