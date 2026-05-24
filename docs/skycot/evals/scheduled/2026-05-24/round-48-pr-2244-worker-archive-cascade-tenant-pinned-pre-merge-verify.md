# Round 48 — PR #2244 `mission-worker-archive-cascade-tenant-pinned` PRE-MERGE anti-theater verify

**Date**: 2026-05-24 (UTC) / 2026-05-25 (Melbourne)
**Round**: 48
**Pillar / Claim**: Claim 6 (Multi-tenant by construction) — tenant-bind sweep cluster downstream of #2167 / #2168 / #2177 recipe.
**Verdict**: 🟢🟢 **PRE-MERGE PASS — P1 substrate contract clean, ready to admin-merge.**

## Setup

PR head OID `d86008787` (codex worker, opened 2026-05-24 18:08 UTC). 130 additions / 3 deletions across 2 files:

- `crates/argo-store/src/worker_archive.rs` (+6 / -3) — production
- `crates/argo-store/tests/worker_archive_cascade_tenant_pin.rs` (+124) — new integration test

Verification commands:

1. `git fetch origin pull/2244/head:pr-2244-head` — pin PR head locally.
2. `git worktree add /tmp/pr-2244-verify pr-2244-head` — isolated cold-build worktree.
3. `CARGO_TARGET_DIR=/tmp/pr-2244-verify-target cargo test -p argo-store --test worker_archive_cascade_tenant_pin` — cold build + run.
4. Independent grep of production SQL (`worker_archive.rs:archive_worker` body) vs `origin/main` baseline.

## Observed

### Production fix (SQL bind sweep)

The four `DELETE` statements in `archive_worker()` now each carry the tenant filter that was previously missing on 3 of 4:

| Table | `origin/main` SQL | PR #2244 SQL | Binding |
|---|---|---|---|
| `duty` | `DELETE FROM duty WHERE worker_id = ?` | `DELETE FROM duty WHERE worker_id = ? AND tenant_id = ?` | `worker_id` + `tenant_id` (TEXT) |
| `role` | `DELETE FROM role WHERE worker_id = ?` | `DELETE FROM role WHERE worker_id = ? AND tenant_id = ?` | `worker_id` + `tenant_id.as_bytes()` (BLOB) |
| `persona` | `DELETE FROM persona WHERE worker_id = ?` | `DELETE FROM persona WHERE worker_id = ? AND tenant_id = ?` | `worker_id` + `tenant_id` (TEXT) |
| `worker` | `DELETE FROM worker WHERE id = ? AND tenant_id = ?` | (unchanged — already tenant-bound) | `id` + `tenant_id` |

The role binding correctly uses `tenant_id.as_bytes()` to match the BLOB tenant column in the `role` table (consistent with existing schema heterogeneity between text/blob tenant columns surfaced by the earlier #1735 / #2184 work).

### Integration test (`worker_archive_cascade_tenant_pin.rs`)

The test constructs a deliberately-synthetic UUID clash: the same `WORKER_ID` row populated across persona / role / duty for **both** TENANT_A and TENANT_B, then archives only TENANT_A's worker. Post-archive assertions:

```
text_child_rows("persona", TENANT_A) == 0   ← tenant A's row DELETED
text_child_rows("persona", TENANT_B) == 1   ← tenant B's row SURVIVES  (load-bearing negative)
role_rows(TENANT_A) == 0
role_rows(TENANT_B) == 1                    ← load-bearing negative
text_child_rows("duty", TENANT_A) == 0
text_child_rows("duty", TENANT_B) == 1      ← load-bearing negative
live_workers (TENANT_A) == 0
archive_rows (TENANT_A) == 1
```

The three `TENANT_B == 1` assertions are load-bearing — they would fail on `origin/main` because the old `DELETE FROM <table> WHERE worker_id = ?` would wipe both tenants' rows in a single statement. They prove the SQL filter is doing real work, not just passing because the archive succeeded.

### Cold cargo result

```
Finished `test` profile [optimized + debuginfo] target(s) in 1m 09s
running 1 test
test archive_cascade_deletes_only_matching_tenant_child_rows ... ok
test result: ok. 1 passed; 0 failed
```

Cold build 1m 09s; test 0.00s (in-memory sqlite).

## Anti-theater check

| Failure mode | How this PR refuses it |
|---|---|
| Test re-writes assertions green | The 3 `TENANT_B == 1` rows are positive-existence assertions that DEMAND a non-deletion; cannot pass on main's SQL. |
| Production fix is comment-only | Verified by independent grep of `worker_archive.rs:133-152` against `origin/main:125-145` — 3 SQL strings each gain `AND tenant_id = ?` + one bind. |
| Schema-vs-SQL drift hides bug | Test seeds the real role-table BLOB schema (`tenant_id BLOB NOT NULL, name TEXT NOT NULL, PRIMARY KEY (tenant_id, name)`) and binds `.as_bytes()` symmetrically — same shape the production code does. |
| Scope creep | Diff stays inside `argo-store` crate (no boundary breakage); 6 LOC production + 124 LOC test. Right-sized. |
| `worker` delete order vs cascade | All 4 deletes run inside one `tx` (`worker_archive.rs:111-159`); commit at line 159; partial-state failure mode covered by the existing transaction wrapper. |

The brief's framing (cluster-3 of the tenant-bind sweep downstream of #2167's tenant-blind cross-tenant audit) is correct — `worker_archive` was sitting in the same shape as the audit identified for skill_share / session-routes / orchestrator handoff, and PR #2244 closes it with the canonical additive-WHERE pattern.

## Gap / follow-up brief candidates

None new from this PR. The downstream sweep continues — PRs #2245 (elevated-view-grant-revoke) and #2246 (scim-user-update) are sibling-shape; next evaluator round should anti-theater-verify those once they reach IN_REVIEW maturity.

## Verdict

🟢🟢 **PRE-MERGE PASS — admin-merge eligible. Recommend conductor land in next tick.** This is the **25th anti-theater-honest substrate contract verification this week** (counting since round 41).
