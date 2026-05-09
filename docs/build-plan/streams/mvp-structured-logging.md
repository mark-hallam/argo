# mvp-structured-logging — `tracing` scope wraps tenant_id/worker_id/session_id

**Owner**: claude. **Depends on**: nothing. **Tier 2 MVP — observability gate.**

**Authored against**: origin/main 7fdbbdb8 on 2026-05-10.

## Why

Today many `tracing::info!` / `tracing::warn!` emissions lack the per-request context (tenant_id, worker_id, session_id), which makes log search painful in multi-tenant deployments. v1.0 needs `tracing::Span` propagation so every emission inside a request scope auto-tags those fields.

## Approach (concrete)

1. **Span wrapper at gateway entry**: in `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs` (after bearer-auth resolves principal), open a `tracing::info_span!("request", tenant_id = %principal.tenant_id, principal_id = %principal.id, route = %route, request_id = %request_id_from_header)` and `.in_scope()` the dispatch.
2. **Worker-loop / agent-runner spans**: in `crates/argo-agent-runner/src/turn.rs` and `crates/argo-cli/src/handlers/work.rs` / `work_loop.rs`, wrap the per-turn / per-job loop body with `info_span!("turn", session_id = %sid, worker_id = %wid)`.
3. **Field convention**: standardize on snake_case field names: `tenant_id`, `worker_id`, `session_id`, `request_id`, `route`, `principal_id`, `audit_kind`. Document in `docs/skycot/RUNBOOK_LOGGING.md` (NEW, ≤ 200 LOC).
4. **Consumers**: do NOT replace existing `info!`/`warn!`/`error!` call sites — they automatically inherit span fields when emitted inside a span. The brief is purely about ADDING the span wrappers.
5. **Tests**: 1 unit test using `tracing-subscriber::fmt::TestWriter` capturing emissions and asserting field presence.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs` — narrowly: 1 span open
- `crates/argo-agent-runner/src/turn.rs` — narrowly: 1 span open in run loop
- `crates/argo-cli/src/handlers/work.rs` and `work_loop.rs` — narrowly: 1 span open each
- `crates/argo-cli/tests/structured_logging_smoke.rs` (NEW, ≤ 200 LOC)
- `docs/skycot/RUNBOOK_LOGGING.md` (NEW, ≤ 200 LOC)

## Files you must NOT touch

- Existing `info!`/`warn!`/`error!` call sites (those inherit span context automatically)
- Tracing init / subscriber setup beyond what's needed for the test

## Acceptance criteria

- [ ] Span at gateway entry with tenant_id, principal_id, route, request_id
- [ ] Span at agent-runner turn with session_id, worker_id
- [ ] Span at work-loop with session_id
- [ ] Test asserts emitted JSON log line contains the expected fields
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] No public API changes

## Branch + PR

- Branch: `mvp-structured-logging`
- PR title: `tracing: scope tenant_id/worker_id/session_id on every emission (mvp-structured-logging)`
