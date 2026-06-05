# ops — fix 2 pre-existing pre-merge-gate reds on main

**Owner**: unclaimed (ops / gate-hygiene)
**Depends on**: none.
**Authored against**: origin/main `357cacec7` on `2026-06-05`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-fix-pre-existing-gate-red-catalog-and-loc`.

## Problem

`scripts/conductor-pre-merge-gate.sh` is **RED on main** for two reasons that
are unrelated to whatever PR is being merged — so the conductor has to keep
boundary-noting + skipping them on every merge. Both are recent UF/sprint drift.
Fix both so the gate goes green again.

### Red 1 — routes-catalog drift (gate 5b)

`POST /api/v1/enterprise-inquiry` is dispatcher-registered (handler
`crates/argo-cli/src/handlers/gateway/routes_enterprise_inquiry.rs`, added by
**#2697** / sprint WS6-2, commit `b32525458`) but is **missing from the catalog**
in `crates/argo-cli/src/handlers/gateway/routes_catalog.rs`. The
`check-routes-catalog-complete` gate fails:

```
/api/v1/enterprise-inquiry    (dispatched: POST)
```

**Fix:** add the matching tuple to the `/api/v1/*` block in `routes_catalog.rs`,
following the existing shape `("POST", "/api/v1/enterprise-inquiry", "<desc>")`.
Description should reflect the real endpoint (public enterprise expression-of-
interest capture → persisted + audited lead). Place it sensibly among the
neighbouring public/marketing routes.

### Red 2 — LOC ceiling (gate 3)

`crates/argo-store/src/lib_tests.rs` is **505 LOC**, 5 over the 500 hard
ceiling. The overage was introduced by **#2714** (UF A6 — `spend_ledger.worker_id`
v90 migration tests). The MVP soft-ceiling-550 window has expired (2026-05-26), so
500 is the live hard ceiling.

**Fix:** split a cohesive block of tests out of `lib_tests.rs` into a sibling
test module so the file lands **≤500 LOC** (aim for headroom, e.g. ~470, not 499).
Prefer moving the A6/spend-ledger or another self-contained group into a new
`crates/argo-store/src/lib_tests_<group>.rs` wired via `#[cfg(test)] mod ...;` (or
the crate's existing test-module convention — grep how `lib_tests.rs` is already
included). Pure test move, no behaviour change.

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_catalog.rs` (one tuple add)
- `crates/argo-store/src/lib_tests.rs` (+ one NEW sibling `lib_tests_*.rs` it moves
  tests into, + the `mod` wiring line wherever `lib_tests` is declared)
- this brief's BUILD_STATUS.md row only

## Verification

- `bash scripts/conductor-pre-merge-gate.sh --skip-clippy` → gate 3 (LOC) and gate
  5b (routes-catalog) both **PASS** (was the only two reds). Paste the gate 3 +
  5b lines in the PR body.
- The moved tests still compile + pass: `cargo test -p argo-store` (lib_tests) and
  `cargo test -p argo-cli routes_catalog` (or the catalog test) — crate-scoped.
  ⚠️ **Memory caution:** the Mac swarm is on 1 slot due to memory pressure; another
  worker may be mid-compile. Run crate-scoped tests (not a full workspace build),
  and use your own per-brief target dir.
- No `src/` behaviour change beyond the catalog tuple + the test move.

## Drift / safe-git (inherited)

- Grep before adding; no new pub types (catalog tuple + test move only).
- `git add` explicit files only; rebase-before-push; never `git reset` shared main.
- No FEATURE_MATRIX change (no capability ships — pure hygiene).
