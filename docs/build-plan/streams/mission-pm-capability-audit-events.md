# Stream P0-4 — audit events for capability bind + dynamic grant

**Owner**: codex.
**Depends on**: none.
**Authored against**: origin/main `80ffa124` on `2026-05-19`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-pm-capability-audit-events`.
**Source**: skycot-pm-worker REPORT.md §5.2 + §11 P0 #4.

## Scope

`CapabilityEngine::bind()` and `CapabilityEngine::grant_dynamic()` (`crates/argo-capability/src/lib.rs`) mutate the live authorization state but emit **zero audit rows**. Tool dispatches are audited, but the dispatches' upstream authorization edits are silent — an operator can re-bind a principal to a higher-privilege role and the audit chain shows nothing until a post-bind dispatch fires.

This brief adds two audit kinds — `RoleBindingChanged` and `DynamicCapabilityGranted` — and wires `argo-audit::AuditSink` into the engine via an optional setter (so existing engine constructors keep working in tests without an audit sink). Every `bind()` / `unbind()` / `grant_dynamic()` / `revoke_dynamic()` call emits a row with `(principal, role_or_capability, actor, at)`.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-audit/src/event.rs` — narrowly scoped: add two `AuditKind` variants (`RoleBindingChanged`, `DynamicCapabilityGranted`). Match the existing variant shape; add to the `as_str` dispatch + the `KNOWN_KINDS` test list.
- `crates/argo-audit/src/event_tests.rs` — extend the kind-count test (it currently asserts `KNOWN_KINDS.len() == 17` per `REPORT.md §6.1`; bump to 19 and add round-trip cases for the two new kinds).
- `crates/argo-capability/src/lib.rs` — add `pub fn with_audit_sink(self, sink: Arc<AuditSink>) -> Self` builder; thread an `Option<Arc<AuditSink>>` field into `CapabilityEngine`. In `bind`, `unbind`, `grant_dynamic`, `revoke_dynamic`: if sink is `Some`, emit the appropriate audit row with payload `{principal, role_or_capability, actor, at}`. Failures to write are logged via `tracing::warn!` but never block the mutation (audit is non-load-bearing on the critical path).
- `crates/argo-capability/Cargo.toml` — narrowly scoped: add `argo-audit` to `[dependencies]`. Verify no cycle (audit must not depend on capability).
- `crates/argo-capability/src/lib_tests.rs` (or existing) — coverage: bind with audit sink emits one row; bind without sink is silent (back-compat).

## Files you must NOT touch

- Any tool dispatch code — that audit is already wired.
- The `Principal` / `CapabilityRole` types — this is engine-layer only.
- Anything in `argo-tool`, `argo-cli` (no caller changes; the audit threading is opt-in via the builder).

## Acceptance criteria

- [ ] Two new `AuditKind` variants: `RoleBindingChanged` and `DynamicCapabilityGranted`. Both serialise to snake_case strings consistent with existing variants.
- [ ] `CapabilityEngine::with_audit_sink(sink)` builder attaches an audit sink without breaking existing constructors.
- [ ] When sink is attached, `bind`, `unbind`, `grant_dynamic`, `revoke_dynamic` each emit one audit row with payload `{principal, role_or_capability, actor, at}` (actor = optional, defaults to `"system"` when caller doesn't pass it — extend the function signatures to accept `Option<&str> actor`).
- [ ] When sink is not attached, all four functions behave identically to current code (back-compat regression test).
- [ ] Audit write errors do NOT abort the mutation; they emit `tracing::warn!` and the engine state remains updated.
- [ ] `event_tests.rs::KNOWN_KINDS` test updated; round-trip serde cases for both new kinds.
- [ ] No cyclic dependency: `cargo metadata` shows `argo-audit` not depending on `argo-capability`.
- [ ] `cargo clippy -p argo-audit -p argo-capability --all-targets -- -D warnings` clean.
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean — claim explicitly in PR body.
- [ ] `cargo test -p argo-audit -p argo-capability` passes.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `codex-mission-pm-capability-audit-events`
- PR title: `capability: audit row on bind + dynamic grant (mission-pm-capability-audit-events)`

## Notes for the implementer

- The "actor" parameter is critical for forensics — "alice@acme bound principal=bob to role=admin" is the question we want to answer. If callers don't have an actor in scope, they must pass `Some("system")` explicitly; never default to `"unknown"`.
- Do NOT introduce an audit-sink trait — `argo-audit::AuditSink` is concrete and already widely used. The builder takes the concrete type.
- The capability engine has no tenant scoping today; the audit row's `tenant_id` is captured via the `AuditSink`'s tenant field (the caller is responsible for constructing the sink with the right tenant). Document this assumption in the function rustdoc.
- Add a small integration test that drives `bind() → dispatch → unbind() → dispatch` and asserts the audit chain has `RoleBindingChanged → ToolCall → RoleBindingChanged → RbacDeny`. This proves the chain is contiguous and post-hoc audit can reconstruct the authorization timeline.

## Out of scope

- Per-tenant audit-routing semantics (cross-tenant capability binds — not currently expressible).
- A CLI surface for browsing capability-bind history (`argo capability log`) — separate brief.
- Emitting an audit row on `add_role` / `remove_role` — role definitions are static config, not runtime mutations. If the policy file changes, a separate audit path (config-reload) handles it.
