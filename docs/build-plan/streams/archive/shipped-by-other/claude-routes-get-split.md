# claude-routes-get-split — split routes.rs (500 LOC) into per-domain GET siblings

**Owner**: claude. **Depends on**: nothing. **Phase A architectural-prep — 9th split in proven pattern (#851/#882/#909/#910/#914/#915/#932/#935).**

## Why

`crates/argo-cli/src/handlers/gateway/routes.rs` is at the **500-LOC ceiling exactly** (HARD RULE 6). Every future GET route addition is BLOCKED until this is split. Most immediate consumer: `mission-gateway-bearer-auth-get-routes` (Phase A follow-up to #946).

The file currently holds:
- `dispatch_route` — top-level GET router (~40 inline `req.starts_with("GET ...)` arms)
- 37 GET-route response builders inline
- The trailing fall-through to `dispatch_post` and `routes_orchestrator::dispatch_route` (sub-dispatchers already extracted in earlier ticks)

POST dispatch was already split into `routes_dispatch_post.rs` (#932). This brief mirrors the same surgery for GET.

## Approach (concrete)

**Pure refactor — zero behaviour change.** Split following the exact pattern of #932 (routes-post-split):

1. **NEW `routes_dispatch_get.rs`** (~280 LOC) — top-level `pub(super) async fn dispatch_get(req, store) -> Option<(&'static str, String)>` with the simple inline GET handlers (root `/`, `/version`, `/health/*`, `/verticals`, `/routes`, `/audit/recent`, `/messages/recent`, etc.). Returns `Some(resp)` on a match, `None` otherwise — same shape as `dispatch_post`.

2. **NEW `routes_dispatch_get_admin.rs`** (~150 LOC, if needed) — admin-style GETs that import their handlers from sibling modules (`routes_audit::*`, `routes_costs_by_day::*`, etc.). Carve here only if `routes_dispatch_get.rs` would otherwise exceed 350 LOC.

3. **Modify `routes.rs`** — replace the 37 inline GET arms with a single `if let Some(resp) = dispatch_get(req, store).await { return resp; }` call near the top of `dispatch_route`. Net delta should drop routes.rs from 500 LOC to ≤ 250 LOC.

4. **Public API stable** — every existing GET response builder either (a) was already in a sibling module (just thread the import), or (b) moves to the new `routes_dispatch_get.rs` and its callers (none in tests, all internal) are updated.

5. **mod.rs**: add `mod routes_dispatch_get;` line (mirrors line 28's `mod routes_dispatch_post;`).

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_dispatch_get.rs` (NEW, ≤ 350 LOC)
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_get_admin.rs` (NEW, ≤ 200 LOC, only if first file would exceed budget)
- `crates/argo-cli/src/handlers/gateway/routes.rs` — narrowly: delete the 37 GET arms, add 1 forwarding call. Should drop to ≤ 250 LOC.
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly: add 1 (or 2) `mod ...;` decls

## Files you must NOT touch

- Any `routes_*.rs` sibling other than the dispatch hub — handlers stay where they are
- `routes_dispatch_post.rs` — POST side is already split and untouched by this brief
- `routes_orchestrator.rs` — the `/orchestrator/*` GETs are already extracted via its own dispatcher
- Any test file (the existing route tests in siblings should pass unchanged through the new dispatcher)

## Acceptance criteria

- [ ] `routes.rs` drops to ≤ 250 LOC (was 500)
- [ ] `routes_dispatch_get.rs` ≤ 350 LOC (or ≤ 200 + admin sibling at ≤ 200)
- [ ] `dispatch_get` returns `Option<(&'static str, String)>` mirroring `dispatch_post` shape exactly
- [ ] All existing tests pass without changes
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean (boundary flakes outside may-touch list are acceptable per CLAUDE.md HARD RULE 2)
- [ ] `gh pr view --json mergeable` returns MERGEABLE post-push

## Branch + PR

- Branch: `claude-routes-get-split`
- PR title: `gateway: split routes.rs GET dispatch into per-domain sibling (claude-routes-get-split)`

## Notes for the implementer

- This is the **9th split in the proven pattern**. Read `routes_dispatch_post.rs` (290 LOC) end-to-end first; your file should be ~95% similar in structure.
- The 37 GET arms in `routes.rs` are trivial-to-mechanical — `if req.starts_with("GET /foo ") { return foo_response(...); }`. Just move them across.
- Watch for parameterised arms (`req.starts_with("GET /verticals?")` etc.) — those use `strip_prefix` and need careful preservation.
- This unblocks `mission-gateway-bearer-auth-get-routes` (the #946 read-side follow-up). Conductor will queue that as TODO once this lands.
