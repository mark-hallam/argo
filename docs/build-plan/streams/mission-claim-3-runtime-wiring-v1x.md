# mission-claim-3-runtime-wiring-v1x — wire argo-routing::TieredRouter into turn.rs (v1.x post-launch)

**Owner**: claude. **Sizing hint**: cross-crate integration + migration + E2E test.
**Depends on**: existing `argo-routing` crate + `RoutingPolicy::Tiered` config type in `argo-types` (both already shipped).
**Authored against**: origin/main `ff02a823` on `2026-05-19`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-claim-3-runtime-wiring-v1x`.
**Source**: 2026-05-18 audit P1 finding #1 — `argo-routing` crate exists but is NOT WIRED into the agent loop. Only consumer is the `argo route inspect|bench` CLI. Tier-aware model selection is paper-only.

**Operator decision (2026-05-18)**: "ship it" — accept claim 3 PAPER → PROVEN-by-half (tier-routing only) as v1.x scope. Inter-provider fallback is a separate brief.

## REWRITTEN 2026-05-19 by conductor

**Why rewritten**: The 2026-05-18 brief was BLOCKED by a claude worker with 5 structural drifts vs actual code (verified in BUILD_STATUS row). Key reality the worker surfaced:

1. **Actual TieredRouter API**: `TieredRouter::new(Arc<dyn Transport>)` + `async fn route(&self, ctx: &RoutingContext<'_>) -> ArgoResult<RoutingDecision>` (`crates/argo-routing/src/tiered.rs:60,80`). There is no `from_manifests` constructor or `select()` method.
2. **Actual integration point**: provider/manifest load happens in `argo-agent-runner::turn::run_turn_with_events_inner` at `turn.rs:193` via `load_manifest(cwd, &session.provider)` — NOT in `dispatch.rs` (which is tool dispatch).
3. **TieredRouter scope**: it's a *tier classifier* that picks a model within a session's chosen provider — NOT inter-provider fallback. Its `fallback_to_default_on_error` flag fires on classifier errors and falls back to the session's default model.
4. **Schema gap**: `SessionHeader` has `provider: String` + `model: String` but no `routing_policy` column. Tier routing needs a policy to read.
5. **No shared test stub for `Transport`**: there are private `LoopForeverTransport` / `MockTransport` / `RecordingTransport` inside individual `#[cfg(test)]` modules; nothing reusable.

The rewrite **narrows scope** to the worker's structurally-correct (a) suggestion: wire `TieredRouter::route()` at `turn.rs:193` for tier-aware model selection within a single provider, gated by a new `session.routing_policy` column. **Inter-provider fallback is explicitly out of scope** and will be authored as a sibling brief `mission-provider-fallback-on-error-v1x` when the swarm picks it up.

This closes the cost-tier half of mission claim 3. The fallback half stays paper for v1.0 launch (operator-locked decision per 2026-05-18).

## Scope

P1 v1.x. Wire `TieredRouter::route()` into the per-turn provider-selection sequence in `turn.rs`. Sessions that opt into a `RoutingPolicy::Tiered` policy will get model selection routed through the tier classifier; sessions with no routing policy (the v1.0 default) behave exactly as today.

### What lands here

1. **Migration 117**: `ALTER TABLE session ADD COLUMN routing_policy TEXT NULL` — JSON-serialized `RoutingPolicy`. NULL means "no policy; use session.model directly" (back-compat for every existing session).
2. **`SessionHeader` + `SessionHeaderRow` field**: `pub routing_policy: Option<RoutingPolicy>`. Read on session load; serialize/deserialize via serde_json against the existing `argo_types::RoutingPolicy` enum.
3. **`SessionStore::set_routing_policy(&self, session_id, policy: Option<RoutingPolicy>) -> Result<()>`**: writer for the new column.
4. **Wire into `turn.rs:193`**: after `load_manifest(...)` and before `let pricing = price_for(&manifest, &session.model);` —
   - If `session.routing_policy` is `Some(RoutingPolicy::Tiered(tiered))`: build a classifier `Transport` via `build_transport_for_model(&manifest, &tiered.classifier_model)` (helper extracted from existing `build_transport`), construct `TieredRouter::new(Arc::new(classifier_transport))`, build a `RoutingContext` from the current turn state (session_id, turn number, messages, user message, tools, deadline, policy=session.routing_policy), call `router.route(&ctx).await?`, override the model for this turn with `decision.model`.
   - If `routing_policy` is `None` or `RoutingPolicy::Default`: behaviour identical to today (just use `session.model`).
   - Persist the resulting `RoutingDecision` via the existing `AuditKind::RoutingDecision` variant (already in `argo-audit::event`); attribute it to this session+turn.
5. **Integration test**: `crates/argo-agent-runner/tests/tiered_routing_smoke.rs` (NEW) — drives a fake-tenant session with a stub classifier transport that always classifies as `"complex"`; asserts the dispatched model matches the `complex` tier's model, NOT the session's default model.
6. **Operator doc**: `docs/skycot/operator/tiered-routing.md` (NEW) — how to set a tiered policy on a session via `argo session set-routing-policy --tiered ...` (CLI command added in this brief; thin wrapper over `SessionStore::set_routing_policy`) + interpretation of `RoutingDecision` audit rows.

LOC budget: ~600 (≈250 prod migration+store+turn-wire + ≈200 test+stub Transport + ≈150 CLI+docs).

## Files you may touch

- `crates/argo-store/migrations/117__session_routing_policy.sql` (NEW). ≤10 LOC.
- `crates/argo-store/src/migrations/v117.rs` (NEW). ≤20 LOC.
- `crates/argo-store/src/migrations/registry.rs` — register slot 117 in the right per-domain sibling. ≤5 modified LOC.
- `crates/argo-store/src/models.rs` — add `routing_policy: Option<RoutingPolicy>` to `SessionHeader` + `SessionHeaderRow`; update `From<SessionHeaderRow> for SessionHeader` to decode the JSON. ≤30 modified LOC.
- `crates/argo-store/src/store_session.rs` (or wherever `SessionStore::load_session_header` lives — verify) — read the new column on load. ≤20 modified LOC.
- `crates/argo-store/src/store_session_routing.rs` (NEW) — `SessionStore::set_routing_policy(&self, session_id, policy: Option<RoutingPolicy>) -> ArgoResult<()>` + 2 unit tests (set then read; set-None clears). ≤80 LOC.
- `crates/argo-agent-runner/Cargo.toml` — add `argo-routing` workspace dep. ≤4 modified LOC.
- `crates/argo-agent-runner/src/turn.rs` — wire `TieredRouter::route()` after `load_manifest`, before `price_for`. Branch on `session.routing_policy.as_ref()`. ≤80 modified LOC.
- `crates/argo-agent-runner/src/turn_routing.rs` (NEW) — helper module containing `build_classifier_transport(&manifest, &model_id) -> ArgoResult<Box<dyn Transport>>` + `build_routing_context_from_turn(...)`. ≤120 LOC. Extracted to keep `turn.rs` under 500 LOC.
- `crates/argo-agent-runner/tests/tiered_routing_smoke.rs` (NEW) — E2E. Define a `TestClassifierTransport` impl inline; assert decision matches tier. ≤180 LOC.
- `crates/argo-cli/src/cli_session_cmds.rs` (or new sibling) — add `SessionCmd::SetRoutingPolicy { id, tiered: Option<...>, default: Option<String>, none: bool }` variant. ≤40 modified LOC.
- `crates/argo-cli/src/handlers/session_routing.rs` (NEW) — handler that parses the CLI args, builds a `RoutingPolicy`, calls `SessionStore::set_routing_policy`. ≤80 LOC.
- `docs/skycot/operator/tiered-routing.md` (NEW). ≤100 LOC.

## Files you must NOT touch

- `argo-routing` crate internals — pure consumer.
- Provider manifest TOMLs (`manifests/providers/*.toml`) — read-only consumer.
- `argo-agent-runner::dispatch.rs` — tool dispatch surface, not provider selection. The original brief mistakenly named this; it is NOT the wiring point.
- `Transport` trait or any provider transport impl — pure consumer.
- Inter-provider fallback logic — out of scope; future brief `mission-provider-fallback-on-error-v1x` will own it.

## Acceptance criteria

- [ ] Migration 117 runs cleanly forward (`cargo test -p argo-store migration_upgrade`).
- [ ] `SessionHeader::routing_policy` serialises round-trip; NULL → None is back-compat.
- [ ] `SessionStore::set_routing_policy(session, Some(Tiered{...}))` then re-load returns the same policy.
- [ ] `SessionStore::set_routing_policy(session, None)` clears the policy.
- [ ] In `turn.rs::run_turn_with_events_inner`, when `session.routing_policy` is `Some(RoutingPolicy::Tiered)`, `TieredRouter::route()` is consulted and its `RoutingDecision.model` overrides the model used for this turn.
- [ ] When `routing_policy` is `None`, dispatch behaviour is byte-identical to today (verified by snapshot test against `turn_tests`).
- [ ] `AuditKind::RoutingDecision` audit row is emitted on every routed turn, carrying session_id, turn number, tier, selected model, and elapsed_ms.
- [ ] `crates/argo-agent-runner/tests/tiered_routing_smoke.rs` passes — stub classifier returns "complex" → assert model = tier-3 model.
- [ ] `argo session set-routing-policy <session-id> --tiered ...` CLI sets the policy; `argo session set-routing-policy <session-id> --none` clears it.
- [ ] `cargo clippy -p argo-store -p argo-agent-runner -p argo-cli --all-targets -- -D warnings` clean.
- [ ] `bash scripts/workspace-clippy-fast-fail.sh` clean.
- [ ] Every modified file < 500 LOC.
- [ ] `docs/skycot/operator/tiered-routing.md` documents the policy shape, the CLI, and the audit row.
- [ ] BUILD_STATUS.md row updated with PR number on IN_REVIEW.

## Branch + PR

- Branch: `mission-claim-3-runtime-wiring-v1x`
- Title: `feat(agent-runner): wire TieredRouter::route() into turn.rs + session.routing_policy column (mission-claim-3-runtime-wiring-v1x) — claim-3 cost-tier half PROVEN`

## Notes for the implementer

- The narrowing decision: the original brief tried to land tier-routing AND inter-provider fallback in one PR. That conflated two distinct concerns (one in argo-routing's scope, one orthogonal to it) and reproduced the "no parallel abstractions" anti-pattern flagged in `docs/skycot/DRIFT_PREVENTION.md`. This rewrite handles tier-routing only. Provider-fallback-on-error is a future sibling brief (`mission-provider-fallback-on-error-v1x`) when the swarm picks it up — the design space there is "extend a transport wrapper / supervisor" not "wire another router". Do not blend the two even if it feels close.
- `build_classifier_transport` is a small helper because the classifier_model lives at `RoutingPolicy::Tiered.classifier_model` and may differ from `session.model`. Reuse the existing `build_transport` body if possible (parameterise it by model id rather than duplicating).
- The integration test uses an inline `TestClassifierTransport` — DO NOT introduce a workspace-shared stub Transport in this brief. That's a separate refactor (workers have flagged the gap; let the next person needing it author the refactor).
- `RoutingDecision` audit row should NOT carry the user message content. Just session_id, turn, tier, model, elapsed_ms. The classifier's *input* (the user message) is already in `Message` rows; don't duplicate.
- If `argo-routing` is not yet a dep of `argo-agent-runner` (verify with `grep argo-routing crates/argo-agent-runner/Cargo.toml`): add it. That's a known forward-only add per the original audit.
- Schema migration 117 adds a nullable column — back-compat. Old sessions read as `routing_policy = None`. No data migration needed.

## Hidden constraints

- **No change to provider manifest TOML schema.** Tier metadata that's needed lives in `RoutingPolicy::Tiered` (which is config, not manifest).
- **No inter-provider fallback in this brief.** The brief explicitly does NOT introduce a `Transport`-wrapping fallback supervisor, a `fallback_chain` config, or any "try provider B if A errors" logic. Those land in `mission-provider-fallback-on-error-v1x`.
- **`AuditKind::RoutingDecision` already exists** (verified in `crates/argo-audit/src/event.rs`). Don't add a new variant — emit the existing one.
- **`turn.rs` is near LOC ceiling.** If `turn.rs` is at >480 LOC, extract the routing call into the new `turn_routing.rs` sibling (already in may-touch). Don't push `turn.rs` past 500.
- **Closes the cost-tier half of mission claim 3.** When this lands + smoke test passes + audit row visible, claim 3's cost-tier story is PROVEN. The fallback story remains PAPER until `mission-provider-fallback-on-error-v1x` lands.

## Out of scope

- Inter-provider fallback on `Transport::complete` error (future sibling: `mission-provider-fallback-on-error-v1x`).
- Cost-tier priority routing (e.g. "use tier-1 for $$$ customers, tier-3 for batch") — that's a different `RoutingPolicy` variant; if needed for a customer, author a follow-up brief.
- Workspace-shared `Transport` test stub (cross-cutting refactor; let the next person to need it author it).
- A `argo route check <session>` debug CLI — useful but not load-bearing for claim 3.
