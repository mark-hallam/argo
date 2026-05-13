# claude-gateway-boot-split — extract background-task spawn helpers

**Owner**: claude. **Depends on**: nothing (pure refactor).
**Authored against**: origin/main `d57d3a31` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-gateway-boot-split`.

## Scope

Pure refactor. `crates/argo-cli/src/handlers/gateway/boot.rs` is at **492 LOC** (8 LOC of headroom against the 500-ceiling). This blocks **9 in-queue mission briefs** that need to add even a small wire-up here:

- mission-llm-call-dedup-cache
- mission-gateway-request-audit-log
- mission-audit-chain-watchdog
- mission-gateway-tls-termination-or-proxy-doc
- mission-structured-logging-tenant-worker-session
- mission-anomaly-alerting-notifychannel
- mission-gateway-bind-default-harden
- mission-siem-integration
- (one more — see conductor tick log)

Split target: extract the **four background-task spawn helpers** into a sibling file. They're cohesive (all return `AbortOnDrop`, all take `Arc<SessionStore>`, all log to `tracing`), self-contained, and total ~150 LOC.

## Approach

Extract these four functions from `boot.rs` into a new sibling `crates/argo-cli/src/handlers/gateway/boot_spawns.rs`:

- `spawn_audit_watchdog(store) -> AbortOnDrop` (line 288)
- `spawn_alerts_loop(store) -> AbortOnDrop` (line 306)
- `spawn_cu_screenshot_sweep(store) -> AbortOnDrop` (line 327)
- `spawn_gateway_duty_scheduler(store) -> AbortOnDrop` (line 376)

`AbortOnDrop` itself (line 48) stays in `boot.rs` — it's used by `boot.rs::start` directly. If extraction requires it to be `pub(super)`, do that; do NOT make it `pub`.

Move only the `use` lines needed by the four functions. Anything still used by the remaining `boot.rs` code stays put.

After split:
- `boot.rs` ≤ 350 LOC (target: ~340)
- `boot_spawns.rs` ≤ 200 LOC

Headroom for the next 5+ mission briefs that need to add a small wire-up to gateway startup before either file approaches the ceiling again.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/boot.rs` — remove the 4 spawn functions + their imports.
- `<NEW: crates/argo-cli/src/handlers/gateway/boot_spawns.rs>` — receives the 4 functions verbatim (zero behavior change).
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly scoped: add ONE `pub(super) mod boot_spawns;` line (or `mod boot_spawns;` depending on visibility convention in that file — match existing siblings). Re-export `pub(super) use boot_spawns::{...}` if the call sites in boot.rs need them.
- No other files.

## Files you must NOT touch

- Anything outside `crates/argo-cli/src/handlers/gateway/`.
- `boot.rs::start` itself — only the helper functions move.
- Tests — if any test referenced the helpers via `crate::handlers::gateway::boot::spawn_*`, update the path to `crate::handlers::gateway::boot_spawns::spawn_*`; do NOT rewrite test bodies.

## Acceptance criteria

- [ ] `boot.rs` ≤ 350 LOC after the split.
- [ ] `boot_spawns.rs` ≤ 200 LOC.
- [ ] Zero behavior change — `argo gateway start` boots identically.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean — claim explicitly in PR body.
- [ ] `cargo test -p argo-cli` passes.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-gateway-boot-split`
- PR title: `gateway: extract boot_spawns sibling (claude-gateway-boot-split)`

## Notes for the implementer

- This is mechanical. No new logic. No new tests beyond what already exists.
- Sibling-file pattern: see `crates/argo-cli/src/handlers/gateway/routes_dispatch_get.rs` etc. — declare `mod boot_spawns;` in `mod.rs` and `use crate::handlers::gateway::boot_spawns::*` (or `pub(super) use ...`) wherever boot.rs needs them.
- If a function uses a `use` that's also still needed by remaining boot.rs code, duplicate the `use` line in both files. Don't try to share.
- The split-prep is `*-split`-suffixed so the pre-claim ceiling check auto-skips (see `scripts/check-pre-claim-ceiling.sh:45`).
