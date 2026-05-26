# Round 90 · S1 · PR #2422 gateway store-helpers extract — POST-MERGE anti-theater

- **Date**: 2026-05-26 (UTC) / 2026-05-27 AEST
- **Round**: 90 (scheduled-task `skycot-evaluator-hourly`, ~3h after r89 ship at 15:13Z)
- **Claim/Pillar**: core invariant — file < 500 LOC, single-binary architecture preserved
- **Verdict**: 🟢 **PASS** (P0 closed)
- **PR**: [#2422](https://github.com/mark-hallam/skycot/pull/2422) — `fix: extract gateway store helpers to keep mod.rs under 500 LOC`, head `dbcecfc3c`, MERGED at `2026-05-26T18:09:54Z` as commit `6145082fc`

## Setup

R89 PRE-MERGE PASS'd this PR at head `dbcecfc3c`. POST-MERGE re-verification:

```bash
gh pr view 2422 --json mergeable,headRefOid,state,mergedAt,additions,deletions,files
wc -l crates/argo-cli/src/handlers/gateway/mod.rs crates/argo-cli/src/handlers/gateway/store_helpers.rs
grep -n "store_helpers\|load_tenant_rate_limit_overrides\|store_unavailable\|body_from_request\|session_tool_call_count\|session_spend_micros\|spend_per_session" crates/argo-cli/src/handlers/gateway/mod.rs
git clone --depth 1 --branch main ~/code/argo /tmp/r90-2422-postmerge
CARGO_TARGET_DIR=/tmp/r90-2422-target cargo check -p argo-cli --message-format=short   # cold
```

## Observed (file shape)

| Path | LOC | Status |
|---|---:|---|
| `crates/argo-cli/src/handlers/gateway/mod.rs` | 424 | under 500-LOC ceiling ✅ |
| `crates/argo-cli/src/handlers/gateway/store_helpers.rs` | 89 | new sibling, 6 helpers extracted |
| total | 513 | (was a single 503-line file) |

Caller pattern in `mod.rs`:

- 1 call-site usage at line 302: `if let Ok(overrides) = load_tenant_rate_limit_overrides().await { ... }`
- Re-export at lines 398-401: `pub(super) use store_helpers::{body_from_request, load_tenant_rate_limit_overrides, session_spend_micros, session_tool_call_count, spend_per_session, store_unavailable};`

The re-export preserves the `pub(super)` calling convention so all 20+ external callsites inside the `gateway/` module sub-tree continue to resolve through the same path (`super::helper_name`) they used before — no callsite migration required.

Visibility shift in the extracted file: `pub(in crate::handlers)` (one level wider than `pub(super)`) is required because `store_helpers.rs` lives one module level below `mod.rs` — the broader visibility is then narrowed at the `use` re-export. Functionally equivalent.

## Anti-theater check

Three independent signals confirm the extraction is genuine, not a copy-paste-without-delete:

1. **Definition uniqueness** — `grep "pub(in crate::handlers) async fn load_tenant_rate_limit_overrides"` finds 1 match (in `store_helpers.rs:14`) and 0 matches in `mod.rs`. The function lives in one place.
2. **Cold cargo check** — `CARGO_TARGET_DIR=/tmp/r90-2422-target cargo check -p argo-cli` from a fresh clone finished in **1m 51s** with `Finished dev profile [optimized + debuginfo] target(s)` and zero errors. If both files defined the helpers, rustc would emit `E0428: the name ... is defined multiple times` immediately.
3. **Body inspection of `store_helpers.rs`** — 6 functions: `load_tenant_rate_limit_overrides` (uses `argo_store::tenant_rate_limit::TenantRateLimitStore`), `store_unavailable` (returns 503 envelope tuple), `body_from_request` (HTTP body parser), `session_tool_call_count` + `session_spend_micros` + `spend_per_session` (3 SQL helpers using `sqlx::query_scalar` / `sqlx::query_as`). All match the 6 originals removed from `mod.rs`.

## Gap / follow-up

None. PR #2422 unblocks PR #2407 (skills-marketplace, 4-of-4 demo-MP4 P0), which was previously gate-3 LOC-blocked by `mod.rs` being above the 500-LOC ceiling. **88th anti-theater verification this week.**
