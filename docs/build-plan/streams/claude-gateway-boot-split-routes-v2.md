# claude-gateway-boot-split-routes-v2 — extract router-assembly out of boot.rs (P0 unblocker)

**Owner**: claude. **Sizing hint**: fast-mode candidate — pure mechanical extraction, no behavior change.
**Depends on**: none.
**Authored against**: origin/main `9fd1a96e` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-gateway-boot-split-routes-v2`.
**Source**: 4 claude TODO briefs (`claude-jwt-cookie-security-hardening`, `claude-error-page-design-system`, `claude-pricing-tier-enforcement-runtime`, `claude-support-inbox-spa-page`) are currently skipped by workers because `crates/argo-cli/src/handlers/gateway/boot.rs` is at **497/500 LOC** and every one of those briefs adds a mount-line to boot. Split-prep unblocks all 4 simultaneously.

## Mission alignment

- **Reliability** (CLAUDE.md priority #1) — 500-LOC ceiling is a non-negotiable architecture invariant.
- **🎯 Unblocker** — this is a single split-prep brief that frees 4 downstream P0/P1/P2 briefs to land.

## Scope

P0. Extract router-assembly logic out of `boot.rs::start()` into a sibling file `boot_router.rs`, dropping `boot.rs` to ≤350 LOC (leaving ~150 LOC of headroom for future mount additions).

### Current state

`boot.rs` is 497 LOC. The bulk is `pub(super) async fn start()` at line 73, ending around line 434 (~361 LOC inside one function). Inside `start()` there's a large block that builds an axum `Router::new()` and chains many `.route(path, handler)` calls.

### The split

Extract the router-assembly block from `start()` into a new file:

```rust
// crates/argo-cli/src/handlers/gateway/boot_router.rs (NEW)
//! Router assembly extracted from boot::start to free LOC headroom.
//! Pure-data: takes the assembled `GatewayState` and returns an
//! `axum::Router`. No I/O, no spawns, no side effects.

pub(super) fn assemble_router(state: GatewayState) -> Router {
    Router::new()
        .route("/healthz", get(routes::healthz))
        .route("/dispatch", post(routes_dispatch_post::handler))
        // ... all the existing .route() calls ...
        .with_state(state)
}
```

Then in `boot.rs::start()`:

```rust
let router = boot_router::assemble_router(state);
// ... rest of start() unchanged ...
```

### Discipline

- **No behavior change.** Pure code-move. Same routes, same handlers, same order.
- **`pub(super)` visibility** — `boot_router` is a sibling helper, not a public API.
- **All mount lines move together** — don't half-extract.
- **boot.rs target: ≤350 LOC** — leaves ~150 LOC of headroom.
- **Tests unchanged.** Existing `boot_tests.rs` + `boot_bind_tests.rs` continue to pass without modification.

LOC: ~200 net (mostly moving lines between files).

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/boot.rs` — remove the route-assembly block; call `boot_router::assemble_router(state)`. ≤150 modified LOC (deletion).
- `crates/argo-cli/src/handlers/gateway/boot_router.rs` (NEW) — `assemble_router(state)` function. ≤350 LOC.
- `crates/argo-cli/src/handlers/gateway/mod.rs` — add `mod boot_router;` (or similar). ≤2 modified LOC.
- `crates/argo-cli/src/handlers/gateway/boot_tests.rs` — only if a test needs an updated import path. ≤10 modified LOC.

## Files you must NOT touch

- Any route handler (`routes_*.rs`). Read-only.
- Any non-boot module.
- Behavior — pure mechanical extraction.

## Acceptance criteria

- [ ] `crates/argo-cli/src/handlers/gateway/boot.rs` is ≤350 LOC (verify: `wc -l <file>`).
- [ ] `crates/argo-cli/src/handlers/gateway/boot_router.rs` exists and is ≤350 LOC.
- [ ] `boot_router::assemble_router` returns the same `Router` as before (test: gateway-smoke `argo gateway start` boots cleanly + GET /healthz returns 200).
- [ ] All existing tests pass without modification: `cargo test -p argo-cli boot_`, `cargo test -p argo-cli gateway`.
- [ ] No behavior change asserted by `bash scripts/smoke-skycot.sh` (local scenarios still green).
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] `bash scripts/check-fn-loc.sh | grep gateway/boot.rs` returns no >80-LOC violations (the extracted helpers should each be ≤80 LOC).
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `claude-gateway-boot-split-routes-v2`
- Title: `refactor(gateway): extract router-assembly out of boot.rs into boot_router.rs (claude-gateway-boot-split-routes-v2)`
- Worker verification: crate-scoped clippy + tests + workspace clippy + smoke per SWARM_PROTOCOL.md.

## Unblocks (post-merge)

These 4 TODO briefs all need to add a mount line to `boot.rs`. Currently refused by workers due to LOC ceiling. After this split:

- `claude-jwt-cookie-security-hardening` (P1) — adds Origin + CSRF + logout route mounts
- `claude-error-page-design-system` (P2) — adds maintenance-middleware mount
- `claude-pricing-tier-enforcement-runtime` (P0) — adds quota-middleware + tier-cap-filter mounts
- `claude-support-inbox-spa-page` (P2) — adds support-ticket route mounts

## Hidden constraints

- **`GatewayState` shape stays unchanged.** This brief doesn't touch the state-builder.
- **Don't extract more than the router.** Tempting to also extract spawn-* blocks, but those live in `boot_spawns.rs` already. Just the router.
- **Order matters in axum**: `Router::new().route(...).route(...).with_state(...)` is order-sensitive for matching. Preserve exact order.
- **No re-export from `gateway/mod.rs`** — `boot_router` is internal to the gateway module.
- **Fast-mode justification**: zero architectural decisions; cut-line is obvious (`Router::new()...with_state(state)` block); pure mechanical move.
