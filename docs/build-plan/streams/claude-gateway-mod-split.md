# PRIORITY — split `gateway/mod.rs` under 500 LOC

**Owner**: claude. **Depends on**: nothing (pure refactor, ARCHITECTURAL PREP).

> **WHY THIS IS PRIORITY:** the file is currently **499 LOC** — at
> the project's 500-LOC ceiling — and is blocking
> `codex-duty-scheduler-gateway-wiring` (PHASE A LOAD-BEARING; the
> autonomous-loop mission claim depends on it). Adding
> `spawn_duty_scheduler` boot wiring + the route_layer for
> `mission-gateway-bearer-auth` middleware both push over.
> This is the 8th split in the proven pattern (#851 / #882 / #909 /
> #910 / #914 / #915 / claude-routes-post-split).

## Scope

`crates/argo-cli/src/handlers/gateway/mod.rs` is at **499 LOC**.
Two pending Phase A items both need to add to this file:
- `codex-duty-scheduler-gateway-wiring` — needs to call
  `spawn_duty_scheduler(...)` in the gateway boot path (~30 LOC).
- `mission-gateway-bearer-auth` — needs to wrap every POST router
  branch with the bearer-auth middleware via `Router::route_layer(...)` (~10-15 LOC).

Even doing both at minimum adds pushes the file over 530 LOC. Split first.

This is a pure refactor. Public API stable.

## Approach

Looking at typical gateway/mod.rs structure (likely):
- ~80-100 LOC of imports + use statements + types
- ~150-200 LOC of `build_router()` — the big route-registration function
- ~50-100 LOC of `serve()` / boot helpers (TLS, bind, gracelet shutdown)
- ~50-100 LOC of cross-cutting helpers (error mapping, request-extension wiring)

Cleanest split:
- Keep `gateway/mod.rs` ≤ 300 LOC: pub `serve()` + `build_router()` skeleton + re-exports
- Move route-registration helpers to `gateway/router_build.rs` (NEW)
- Move boot/serve helpers to `gateway/boot.rs` (NEW)
- Move shared error-mapping / extractor glue to `gateway/error_glue.rs` (NEW) if it makes sense

Naming convention: `gateway/<purpose>.rs` (mirrors `routes_orchestrator.rs` /
`routes_audit.rs` siblings already in the same directory).

After:
- `gateway/mod.rs` ≤ 300 LOC — the public façade
- 2-3 NEW sibling files, each ≤ 250 LOC
- Public API stable

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/mod.rs` — reduce to ≤ 300 LOC
- `crates/argo-cli/src/handlers/gateway/router_build.rs` (NEW)
- `crates/argo-cli/src/handlers/gateway/boot.rs` (NEW)
- `crates/argo-cli/src/handlers/gateway/error_glue.rs` (NEW, if needed)
- Any file that imported by the full `handlers::gateway::<fn>` path —
  narrowly: change import path if a symbol moved (should be zero
  edits if `pub use` re-exports are right)

## Acceptance criteria

- [ ] `gateway/mod.rs` ≤ 300 LOC
- [ ] Each new sibling file ≤ 250 LOC
- [ ] No public-API change (every existing call site still compiles unchanged)
- [ ] `cargo build --workspace` clean
- [ ] `cargo test --workspace` — same set of tests pass as before
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Both Phase A blockers (`codex-duty-scheduler-gateway-wiring` +
      `mission-gateway-bearer-auth`) become unblocked: their
      planned additions fit within remaining headroom

## Branch + PR

- Branch: `claude-gateway-mod-split`
- PR title: `refactor: split gateway/mod.rs under 500 LOC budget (claude-gateway-mod-split)`
