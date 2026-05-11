# Mission load-bearing wire — Structured logging with tenant_id + worker_id + session_id on every line

**Owner**: claude. **Depends on**: existing `argo-telemetry` tracing primitives, existing `argo-types::{TenantId, WorkerId, SessionId}` newtypes.
**Authored against**: origin/main `01aa2f5d` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-structured-logging-tenant-worker-session`.

> SOURCE: MISSION_BACKLOG.md Section 1 #33 ("Add structured logging with tenant_id + worker_id + session_id fields on every log line"). **Tier M (observability).** Today tracing events emit ad-hoc fields — a Slack-channel log has `tenant_id`, an LLM-call log has `session_id`, a worker-loop log has `worker_id`, but no log has all three reliably. SREs grepping Loki / Datadog / CloudWatch for "what did tenant X do during session Y on worker Z" can't cross-correlate. This brief makes the three-tuple a load-bearing convention via a tracing-subscriber layer that pulls from `tracing::Span` context.

## Scope

NEW `crates/argo-telemetry/src/span_context.rs` (≤ 220 LOC):

```rust
/// The three-tuple every log line should carry when in a request/work context.
pub struct LogContext {
    pub tenant_id: Option<TenantId>,
    pub worker_id: Option<WorkerId>,
    pub session_id: Option<SessionId>,
}

/// Enter a `LogContext`-bearing tracing span. Returns a guard.
pub fn enter_context(ctx: LogContext) -> tracing::span::EnteredSpan;

/// `tracing-subscriber` layer that injects tenant_id/worker_id/session_id
/// from the current span (or any ancestor) into every emitted event,
/// regardless of whether the event-author explicitly named them.
pub struct ContextInjectorLayer;
```

NEW migration callsites (mechanical, ≤ 20 LOC delta per site, across 8-12 files):
- gateway boot path: enter a context when bearer-token resolves the principal's `tenant_id`
- worker loop: enter a context when a worker claims a brief (`worker_id` from pulse file)
- session-runner: enter a context when a session starts (`session_id` from the new session row)

NEW tracing-subscriber wiring in `argo-telemetry::init_subscriber` (or wherever the subscriber is built):
- Install `ContextInjectorLayer` BELOW the existing event-formatter, so every event gets the three fields automatically.

NEW `[telemetry.context]` config block (`crates/argo-config/src/sections.rs`):
- `enabled: bool = true` (operators can disable for read-only deployments where the SREs don't care)
- `emit_anon_tenant_as: String = "anon"` (the string emitted when `tenant_id` is None — typical anon GETs to /health)

This is NOT in scope: per-tenant log-volume budgets (separate brief); log-line PII redaction (separate brief, even though context-aware redaction will eventually want these three fields); log-sampling rules; OTel trace propagation beyond local-process (separate); structured-log shipping to Loki / Datadog / CloudWatch (separate ops brief).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-telemetry/src/span_context.rs` (NEW, ≤ 220 LOC)
- `crates/argo-telemetry/src/span_context_tests.rs` (NEW, sibling, ≤ 200 LOC) — 7 unit tests (enter-emits-all-three / partial-context-emits-Some+None / nested-ancestor-context-inherits / formatter-includes-fields / enabled-false-zero-overhead / config-emit_anon_tenant_as-honored / SessionId-Option-precedence)
- `crates/argo-telemetry/src/lib.rs` — narrowly scoped: register module + extend `init_subscriber`
- `crates/argo-telemetry/Cargo.toml` — narrowly scoped: confirm `tracing-subscriber` is direct dep (likely already)
- `crates/argo-config/src/sections.rs` — narrowly scoped: ADD `[telemetry.context]` section
- `crates/argo-config/src/defaults.rs` — narrowly scoped
- `crates/argo-cli/src/handlers/gateway/boot.rs` — narrowly scoped: ADD `enter_context` after principal resolution (≤ 8 LOC delta)
- `crates/argo-cli/src/handlers/worker_actions.rs` (or wherever the worker-loop owns its main span — verify via `rg -l 'WorkerId\|worker_loop\|WorkerLoop' crates/argo-cli/src/`) — narrowly scoped: ADD `enter_context` at iteration boundary (≤ 8 LOC delta)
- `crates/argo-runtime/src/session.rs` (or wherever Session::start is — verify via `rg -l 'fn start_session\|Session::new' crates/argo-runtime/src/`) — narrowly scoped: ADD `enter_context` at session start (≤ 8 LOC delta)
- `crates/argo-cli/tests/structured_log_context_smoke.rs` (NEW, ≤ 150 LOC) — integration smoke: boot gateway, hit `/providers` with bearer that maps to tenant_X, parse stderr lines, assert every line emitted during the request carries `tenant_id=tenant_X`

## Files you must NOT touch

- `crates/argo-store/migrations/**` — no schema
- `crates/argo-audit/**` — audit chain is separate; this is just log lines
- Any vertical
- Frontend / web

## Acceptance criteria

- [ ] Every log line emitted DURING a gateway request carries `tenant_id` (or `tenant_id=anon` for unauth GETs)
- [ ] Every log line emitted DURING a worker-loop iteration carries `worker_id`
- [ ] Every log line emitted DURING a session run carries `session_id`
- [ ] Log lines emitted OUTSIDE all three contexts (e.g. boot-time setup) emit `tenant_id=anon worker_id=- session_id=-` consistently
- [ ] The fields are present even when the event-author did NOT explicitly include them (Layer-injection works)
- [ ] `[telemetry.context].enabled = false` → ContextInjectorLayer is NOT registered (true zero-overhead path)
- [ ] 7 unit tests on span_context + 1 integration smoke verifying the three-tuple appears on real log lines through stderr
- [ ] `cargo clippy -p argo-telemetry -p argo-config -p argo-cli -p argo-runtime --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-telemetry -p argo-config -p argo-cli` passes
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-structured-logging-tenant-worker-session`
- PR title: `telemetry: tenant/worker/session context on every log line (mission-structured-logging-tenant-worker-session)`

## Notes for the implementer

- First verify the existing tracing-subscriber wiring shape: `rg -l 'tracing_subscriber::registry\|init_subscriber\|EnvFilter' crates/argo-telemetry/src/`. The layer-injection pattern requires that subscribers be composed via the `tracing-subscriber::layer::SubscriberExt` trait — if the existing wiring uses a flat `tracing_subscriber::fmt::init()`, this brief needs to refactor it to the layered form.
- `ContextInjectorLayer` impl: on each event, walk the current `Span` and its ancestors looking for the three fields. Cache the lookup per-span (use `Span::extensions_mut()` to store a `LogContext` cell on first lookup so subsequent events on the same span are O(1)).
- The three call-sites (gateway-boot / worker-loop / session-start) MUST be the only sites that call `enter_context` directly. All other code should rely on the layer-injection working automatically. Document this in module docs.
- For the "anon" rendering: emit literally `tenant_id=anon` or whatever the operator set in config. Don't omit the field — observability tools differentiate between "field absent" (subscriber broken) and "field=anon" (intentional anon request).
- The integration smoke should set `ARGO_LOG_FORMAT=json` so the assertions can parse JSON log lines for the three fields rather than fragile regex on text. If the format isn't already configurable, that's a follow-up brief — for this brief's smoke, parse the text format.
- Cross-thread propagation: `tokio::spawn` does NOT inherit tracing-span context by default. Wrap any spawned future with `.in_current_span()` from `tracing-futures` when the spawn happens inside a request/work context. The brief's three call-sites must all use `.in_current_span()` for spawned children — that's part of the load-bearing wiring.
- Don't break existing tracing-event call-sites that already pass `tenant_id` / `worker_id` / `session_id` explicitly — the Layer should OVERRIDE the event-author's value with the span-ancestor value when both are present, OR use the event-author's value if more specific. Pick one and document it. (Recommendation: event-author wins when they explicitly named it; Layer fills in only when absent. This preserves "more specific tenant_id at the call-site" overrides for edge cases.)
