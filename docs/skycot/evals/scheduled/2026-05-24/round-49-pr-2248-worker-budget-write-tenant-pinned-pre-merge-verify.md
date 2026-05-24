# Round 49 · PR #2248 `mission-worker-budget-write-tenant-pinned` PRE-MERGE anti-theater verify

- **Date**: 2026-05-24 (UTC) · Melbourne 2026-05-25
- **Round**: 49 (scheduled-task hourly fire, ~1h after r48)
- **Pillar / Claim**: Claim 6 (Multi-tenant by construction) · substrate tenant-bind sweep cluster, sibling-4 of cluster downstream of #2167
- **PR head verified**: `b37f27021` (mission: bind worker budget writes by tenant)
- **Verdict**: 🟢 **PRE-MERGE PASS** (recommend admin-merge next conductor tick alongside #2244/#2245/#2246)
- **Severity**: P1 substrate (no LIVE outage; tenant-isolation hardening + claim-6 contract)

## Setup

```bash
git fetch origin pull/2248/head:pr-2248-head
git worktree add /tmp/pr-2248-verify pr-2248-head
cd /tmp/pr-2248-verify
RUSTC_WRAPPER= CARGO_TARGET_DIR=/tmp/pr-2248-verify-target \
  cargo test -p argo-store --test worker_tenant_pin
```

Cold worktree (not the shared `target/`), independent verifier — no risk of test result borrowing from the conductor's incremental compile cache.

## Observed

```
Finished `test` profile [optimized + debuginfo] target(s) in 5m 27s
     Running tests/worker_tenant_pin.rs
running 2 tests
test worker_presence_touch_binds_worker_tenant ... ok
test worker_budget_helpers_bind_worker_tenant ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; finished in 0.17s
```

5 files changed (+300 / −57 = 243 net LOC): `worker_budget_write.rs` (460 LOC), `worker_presence.rs` (237 LOC), `worker_tenant_pin.rs` (117 new LOC), `sink_chain.rs` (240 LOC), `worker_budget.rs` CLI handler (201 LOC). All under the 500-LOC budget. No clippy run this round (PR body claims `cargo clippy -p argo-store -p argo-cli -p argo-audit --all-targets -- -D warnings` clean; PRE-MERGE gate will validate).

## Anti-theater check (the load-bearing part)

Independent grep of the production diff vs `origin/main`, NOT just trust of the test result:

**`worker_budget_write.rs` — every SQL helper now binds `tenant_id`:**

- `load_policy_value`: `SELECT auto_approval_json FROM worker WHERE id = ? AND tenant_id = ?` (TEXT bind: `tenant_id` direct).
- `write_policy_value`: `UPDATE worker SET auto_approval_json = ? WHERE id = ? AND tenant_id = ?` — `rows_affected() == 0` ⇒ error `"worker {worker_id} not found in tenant {tenant_id}"` (load-bearing — would silently swallow on main).
- `load_spend_totals`: BOTH today + month SUMs now read `WHERE tenant_id = ? AND worker_id = ? AND charged_at >= ?`. Critically uses `.bind(tenant_id.as_bytes())` — this matches the `worker_budget` table's BLOB column type, the SAME pattern as PR #2244's role-table fix verified round 48. Inconsistent type-binding would cause SQLite collation-equality to silently mismatch — which is what would happen if a naïve fix bound the TEXT representation.
- All 4 public helpers (`set_cap`, `clear_cap`, `summary`, `load_spend_totals`) now take `tenant_id: &str` as the leading param.

**`worker_presence.rs` — two-path tenant binding:**

- New `touch_presence_for_tenant(pool, tenant_id, worker_id)`: explicit caller path; SQL `UPDATE worker SET last_active_at = strftime('%s','now') WHERE id = ? AND tenant_id = ?`.
- Legacy `touch_presence(pool, worker_id)`: preserved for backwards-compat by way of an internal `tenant_id_for_worker(pool, worker_id)` lookup — does the SELECT FIRST then the bound UPDATE. The eventual UPDATE is always tenant-bound. *Subtle property*: under hypothetical worker_id collision across tenants, `LIMIT 1` picks one arbitrarily — not a cross-tenant leak (the UPDATE still scopes to the picked tenant) but non-deterministic. Acceptable risk given the worker.id PK is UUIDv7 + the cross-tenant tests (#2244, etc.) have demonstrated UUID-collision is not the failure mode in practice.

**Integration test — load-bearing negative controls (would FAIL on `origin/main`):**

- `worker_budget_helpers_bind_worker_tenant`:
  1. Seeds WORKER_ID in TENANT_A with `daily_budget_usd_micros: 1`.
  2. Calls `set_cap(OTHER_TENANT, WORKER_ID, Some(9), None)` → asserts error contains OTHER_TENANT name.
  3. **Then reads the policy back and asserts it's STILL `1`, not `9`** — would fail on main (old SQL would UPDATE without tenant bind, returning success and writing `9`).
  4. Seeds TWO `worker_budget` rows with same WORKER_ID, different tenant_id (7 micros in A, 13 in OTHER).
  5. Calls `summary(TENANT_A, WORKER_ID, now)` → asserts `today_micros == 7` and `this_month_micros == 7` — would fail on main with `7 + 13 = 20` (no tenant filter on SUM).

- `worker_presence_touch_binds_worker_tenant`:
  1. Seeds WORKER_ID in TENANT_A with `last_active_at = 123`.
  2. Calls `touch_presence_for_tenant(pool, OTHER_TENANT, WORKER_ID)`.
  3. **Asserts `last_active_at == 123` (unchanged)** — would fail on main (UPDATE without tenant filter would bump to `now`).
  4. Calls with TENANT_A → asserts `> 123`.

These are NOT "test rewrites itself green" — they are exploit-shaped seeds with bind-mismatched calls and read-back assertions on the negative path.

## Brief candidates / follow-up

None this scenario — the brief is complete and merges cleanly. Cluster sibling status of the 4 tenant-bind sweep PRs (downstream of #2167):

| PR | Brief | Status (r49) | Round verified |
|---|---|---|---|
| #2244 | mission-worker-archive-cascade-tenant-pinned | OPEN MERGEABLE CLEAN | r48 🟢 |
| #2245 | mission-elevated-view-grant-revoke-tenant-pinned | OPEN MERGEABLE CLEAN | not yet |
| #2246 | mission-scim-user-update-tenant-pinned | OPEN MERGEABLE CLEAN | not yet |
| #2248 | mission-worker-budget-write-tenant-pinned | OPEN MERGEABLE CLEAN | **r49 🟢 (this report)** |

Recommend conductor-Step-5 ships a 4-PR tenant-bind cluster merge in the next claude-conductor tick (T19-08 already flagged this — see SHARED.md). Per-PR `gh pr view <n> --json mergeable` re-check between merges remains mandatory per the `feedback_gh_merge_exit_code_trap` lesson.

**27th anti-theater-honest substrate contract verification this week.**
