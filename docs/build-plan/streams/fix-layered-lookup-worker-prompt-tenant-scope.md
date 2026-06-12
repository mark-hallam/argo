# fix-layered-lookup-worker-prompt-tenant-scope

**Owner**: claude or codex — **WS1 TENANT-ISOLATION → conductor adversarial review before merge, do NOT self-merge.**
**Depends on**: skill-store-list-active-cross-tenant-leak MERGED (#2848 — provides the `*_for_tenant` strict readers to thread through).
**Authored against**: origin/main `ff11283ed` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-layered-lookup-worker-prompt-tenant-scope

## Problem (surfaced by the #2848 adversarial review 2026-06-12 — the LARGER remaining surface)

#2848 closed the cross-tenant skill-read leak on the gateway READ routes,
but the adversarial review confirmed the bigger exposure is still open: the
`LayeredLookup` worker-prompt path still calls the UNSCOPED
`find_active_by_name` / `list_active` —
`crates/argo-skill/src/library.rs:102,109,131,143` reached from
`crates/argo-agent-runner/src/turn_helpers.rs:66`. This is what feeds
skills into a RUNNING teammate's prompt, so on the shared multi-tenant
store a teammate in tenant A can be fed tenant B's active skills — a
cross-tenant read on the dispatch hot path (arguably higher-impact than the
gateway read routes #2848 fixed).

## Fix

Thread the session's tenant (already on `turn_helpers.rs`'s session/ctx —
the same tenant the fence + memory bind) into `LayeredLookup`'s lookups and
switch them to the `*_for_tenant` strict readers #2848 added. This is the
"larger signature change" #2848 deferred: `LayeredLookup` (and
`with_library`) gain a tenant parameter (additive — grep all construction
sites: `turn_helpers.rs:65` is the prod one; tests construct it too). NO
parallel reader — reuse #2848's strict methods. Also sweep the skill WRITE/
admin paths the review named (`routes_post_admin.rs` lock/unlock, session
attach/detach in `routes_session_skills.rs`) — fix in-scope if they take a
tenant cleanly, else name them as a further follow-up in the PR body.

## May touch
- `crates/argo-skill/src/library.rs` (+ tests)
- `crates/argo-agent-runner/src/turn_helpers.rs` (thread tenant)
- the skill write/admin routes IF fixed in-scope

## Acceptance / verifier
1. A worker in tenant A is NEVER fed tenant B's active skill via the
   prompt-layer lookup — cross-tenant negative test at the LayeredLookup
   level (would FAIL on current main).
2. Same-tenant skill-library load still works (the #1682 worker-load path
   regression-pinned green). 3. `cargo test -p argo-skill -p
   argo-agent-runner` green; clippy clean; no new pub types.

~120-200 LOC. **🔴 P1 — the larger cross-tenant skill-read surface, on the
running-teammate dispatch path.**
