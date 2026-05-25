# pitch-truth-boundary-runtime-enforcement

**Owner**: claude.
**Authored against**: origin/main `ac7095d0c` on `2026-05-25` (refreshed via `fix-pitch-truth-boundary-runtime-enforcement-brief-refresh`).
**Validate before claim**: `bash scripts/check-brief-freshness.sh pitch-truth-boundary-runtime-enforcement`.
**Blocked by**: `pitch-truth-boundary-capability-schema` (P-01) — MERGED #2214.

🔴 **P0 v1.0 ship-blocker · Track P (Pitch Agent) · brief P-02 of 8**

Wires P-01's truth-boundary evaluator into the agent runtime so every utterance an agent emits passes through the gate before it leaves the process. This is the brief that makes the capability layer *enforced* rather than *defined*.

**Recipe choice (resolved at brief refresh 2026-05-25)**: Recipe A — call the free function `argo_capability::truth_boundary::evaluate_claim()` directly from the turn-exit point. P-01 shipped `evaluate_claim` as a free function (not a method on `CapabilityEngine`), and the existing `CapabilityEngine` type in `argo-capability` is the RBAC engine — distinct concern, must not be conflated. Recipe A keeps `truth_boundary.rs` untouched (per the original Out-of-scope rule). The "engine" plumbing is the new `AgentDeps` fields described in §2.

## 500-LOC preflight (read before claiming)

Two may-touch files are at-risk of the HARD RULE 6 per-file ceiling. The implementer MUST handle these before adding code:

- `crates/argo-audit/src/event_kind.rs` — currently **500 LOC** (exact ceiling). Adding 3 new `AuditKind` variants with rustdoc will push it over. Split `event_kind.rs` into topic-grouped sibling files (e.g. `event_kind_claim.rs`, `event_kind_workflow.rs`, etc.) FIRST as a separate architectural-prep PR, or extend the may-touch list of this brief to cover the split inline (recommended: prep PR). The `label` / `from_label` / `all` impls in `event.rs` will reference all variants regardless of where they're declared, since they live on the single `AuditKind` enum.
- `crates/argo-agent-runner/src/turn_helpers_run.rs` — currently **472 LOC** (28 LOC headroom). Confine new enforcement code to the new `turn_helpers_truth_boundary.rs` sibling; only modify `turn_helpers_run.rs` if `RoundOutcome` must grow a `declared_claim_classes` field (≤5 LOC delta).

If either ceiling blocks implementation, HARD RULE 6 says BLOCK + ask the conductor for a `claude-argo-audit-event-kind-split` prep brief.

## Why this brief exists

P-01 added the types; this brief adds the call site. Without P-02, the storyboard's three refusal beats (Scenes 2b/2c/4b) cannot happen — the agent would still hallucinate a valuation because no code path is checking. This brief inserts the check into the runtime turn loop at the single exit point where the agent's response becomes visible to the channel.

The operator-locked invariant: the gate is in the *runtime*, not in the *prompt*. A prompt can be talked out of; runtime enforcement cannot.

## Source of truth

- **P-01 brief**: `pitch-truth-boundary-capability-schema.md` — types this consumes (MERGED #2214).
- **P-01 evaluator entry point**: `argo_capability::truth_boundary::evaluate_claim(policy, channel, candidate) -> ClaimEvaluation` at `crates/argo-capability/src/truth_boundary.rs:414`.
- **`AgentDeps`**: `crates/argo-core/src/agent.rs:30` — the only injection point per platform invariants. Note: the existing `pub capability_engine: Arc<CapabilityEngine>` field is the RBAC engine; do **not** repurpose it for truth-boundary state.
- **Turn loop exit point**: `pub(super) fn finalize_turn(outcome: RoundOutcome) -> TurnResult` at `crates/argo-agent-runner/src/turn_helpers_run.rs:464` — called by `run_turn_with_tools` in `turn_helpers.rs:190` when the assistant returns plain text (no further tool calls). This is the "candidate response produced" → "response committed" boundary.
- **Audit emission pattern**: `argo-audit` persists rows via `AuditSink::write(session_id, AuditKind, payload_json, principal)`. Variants are declared in `crates/argo-audit/src/event_kind.rs`; the wire-label + `from_label` + `all` impls live in `crates/argo-audit/src/event.rs`. There is no `AuditEvent::Variant { .. }` enum-with-payload shape — payload is free-form `serde_json::Value`. (The struct named `AuditEvent` in `sink.rs:55` is the broadcast/stream-out shape, not the persisted-row discriminator.)
- **Memory**: `project_pitch_agent_v1_demo.md` — "capability gate covers both paths, audit-logged".
- **Storyboard**: Scene 2b (audit panel slides in showing the block) — proves audit emission is part of this brief.

## Scope

### 1. Add the gate at the turn-exit point

Insert a synchronous gate call into `turn_helpers_run::finalize_turn` (or its caller in `turn_helpers::run_turn_with_tools` immediately before `finalize_turn` is invoked — implementer judgment). The gate runs once per turn, on the assistant's final text — i.e. only when `outcome.tool_calls.is_empty()` and the loop is about to return `TurnResult`.

Sketch (exact location varies — read the file first):

```rust
// Inside turn_helpers::run_turn_with_tools, where finalize_turn(outcome)
// is currently returned:
let candidate = ClaimCandidate {
    text: outcome.text.clone(),
    declared_classes: outcome.declared_claim_classes.clone(), // see §2 on how this lands
};

let eval = argo_capability::truth_boundary::evaluate_claim(
    &deps.active_truth_boundary_policy,
    deps.active_channel_kind,
    &candidate,
);

let final_outcome = match eval.disposition.clone() {
    ClaimDisposition::Allow => {
        emit_claim_allowed(audit, session_id, &eval).await?;
        outcome
    }
    ClaimDisposition::Forbid { refusal_template_id } => {
        emit_claim_blocked(audit, session_id, &eval, &deps.active_truth_boundary_policy.id).await?;
        let refusal = pick_refusal_variant(&deps.refusal_templates, &refusal_template_id)?;
        replace_outcome_text(outcome, refusal)
    }
    ClaimDisposition::Escalate => {
        let approval_id = enqueue_approval(/* … */).await?;
        emit_claim_escalated(audit, session_id, &eval, &approval_id).await?;
        replace_outcome_text(outcome, pending_approval_message())
    }
};
return Ok(finalize_turn(final_outcome));
```

The `Forbid` and `Escalate` branches mutate the outcome's `text` and leave the assistant message row already-persisted unchanged (history shows what the model produced; the channel just delivers the refusal/pending text). If that history-vs-delivered split is awkward, the implementer may rewrite the persisted assistant row in the same transaction — flag the choice in the PR body.

### 2. Wire the new `AgentDeps` fields

`AgentDeps` lives at `crates/argo-core/src/agent.rs:30`. It is `#[derive(Clone)]`; new fields must be `Clone`. Add three new fields (the existing `capability_engine` stays untouched — it is the RBAC engine):

- `pub active_truth_boundary_policy: Arc<TruthBoundaryPolicy>` — resolved at session boot from the worker template manifest. `Arc<…>` because the same policy is shared across the session's many turns.
- `pub active_channel_kind: ChannelKind` — resolved at session boot from the channel adapter that started the session. `Copy` so no `Arc`.
- `pub refusal_templates: Arc<RefusalTemplateRegistry>` — loaded at engine boot. P-01 ships `RefusalTemplate` (singular); the registry type is either new in this brief or already in `truth_boundary_policies.rs` — verify before authoring. If new, place it in `argo-capability` (new sibling module — DO NOT touch `truth_boundary.rs`) or in `argo-agent-runner`.

If any of `active_truth_boundary_policy` / `active_channel_kind` is `None` for a given session (e.g. a backend job with no channel), the gate must short-circuit Allow without panic. Implement either as `Option<…>` fields or as "no-op policy" sentinel constants — implementer choice; document in PR body.

`ClaimCandidate.declared_classes` must come from somewhere. v1 plumbing: extend the `RoundOutcome` struct (in `turn_helpers_run.rs`) with `declared_claim_classes: Vec<String>` populated to `vec![]` by default; P-04 (canonical Q&A skill) will be the first producer that sets it non-empty. If extending `RoundOutcome` causes cross-crate ripple, plumb it as a side-channel on `TurnDispatchCtx` instead.

### 3. Refusal-variant selection helper

`pick_refusal_variant(&registry, &id) -> ArgoResult<String>` — randomized variant pick (stable seed per session for testing). ~15 LOC. Lives in a new helper module under `argo-agent-runner` (e.g. `turn_helpers_truth_boundary.rs`) — DO NOT touch `truth_boundary.rs`. The registry side (looking up `RefusalTemplate` by `template_id`) may be a thin new fn in `argo-capability` outside `truth_boundary.rs` if needed.

### 4. Audit events — add 3 new `AuditKind` variants

Extend `pub enum AuditKind` in `crates/argo-audit/src/event_kind.rs` with three variants (each is a stable wire label; payload is free-form `serde_json::Value`):

- `ClaimAllowed` → label `"claim.allowed"` — payload: `{ tenant_id, policy_id, channel, matched_class?, provenance, candidate_excerpt }`
- `ClaimBlocked` → label `"claim.blocked"` — payload: `{ tenant_id, policy_id, channel, matched_class, candidate_excerpt }`
- `ClaimEscalated` → label `"claim.escalated"` — payload: `{ tenant_id, policy_id, channel, matched_class, approval_id }`

Each new variant requires three edits, all in the `argo-audit` crate:

1. Declaration in `event_kind.rs` (the `pub enum AuditKind` body) with rustdoc.
2. `label` match arm in `event.rs` (`impl AuditKind`).
3. `from_label` match arm in `event.rs` (same impl block).
4. `all` array entry in `event.rs` (the test-list helper).

Tenant_id stamping is enforced by `AuditSink::write` (per the `audit-sink-tenant-guard` CI lint); no payload-side tenant work needed beyond putting it in for human-readability.

Emission goes through the existing `AuditSink::write(Some(session_id), AuditKind::ClaimBlocked, payload, principal)` path. Use the small helper-fn pattern shown in `crates/argo-capability/src/audit_emit.rs` for spawning the write off the hot path if appropriate.

### 5. Provenance propagation

`evaluate_claim` returns a `ClaimEvaluation` with a `ClaimProvenance` (P-01 type). Pass the provenance through into the audit payload so the audit page (B3 design) can show "where this answer came from" — canonical-Q&A hit, approved-memory pull, or generative. v1 P-02 emits the `provenance: ProvenanceSource::Generative` default that P-01's free function returns; P-04 (canonical-Q&A) will become the first producer that surfaces a real provenance, but no producer-side work is in this brief's scope.

### 6. Channel-context resolution

P-02 plumbs `active_channel_kind: ChannelKind` through `AgentDeps` as a per-session config field set at session boot (§2). This brief does **not** touch the `argo-channels` traits. There is no `ChannelAdapter` trait in the current code — channel transports are `NotifyChannel` (approval-notify) and `OutboundChannel` (text-dispatch), and neither is plumbed into the agent-runner turn loop. P-06 (`pitch-skycot-web-embed-channel`) and P-07 (`pitch-meeting-orchestrator-zoom-channel`) are responsible for wiring their channel kinds into session boot when they land.

### 7. Tests

New test file `crates/argo-agent-runner/src/turn_helpers_truth_boundary_tests.rs` (NEW), wired via `#[cfg(test)] #[path = "turn_helpers_truth_boundary_tests.rs"] mod ...;` from `turn.rs` or `lib.rs` per the existing sibling pattern (`turn_tests.rs`, `turn_skills_tests.rs`).

Cases (≥8):

- An allowed claim passes through with a `ClaimAllowed` audit row.
- A forbidden claim is replaced with a refusal variant and emits `ClaimBlocked`.
- An escalated claim queues an approval and emits `ClaimEscalated`.
- Cross-tenant: tenant A's policy does not gate tenant B's session.
- Per-channel: same claim class is `Allow` on `Rehearsal` and `Forbid` on `WebEmbed`.
- Provenance: a candidate with `ProvenanceSource::CanonicalQa` carries through to the audit payload.
- No-policy short-circuit: `None` policy → Allow with no audit row (or with an Allow audit row — implementer choice).
- Refusal-variant determinism: same `template_id` + same `session_id` → same variant (stable seed).

## Files you may touch

- `crates/argo-agent-runner/src/turn_helpers.rs` (insert gate call before `finalize_turn`)
- `crates/argo-agent-runner/src/turn_helpers_run.rs` (extend `RoundOutcome` with `declared_claim_classes` if §2 plumbing requires; the existing `finalize_turn` may also need a thin signature change — implementer judgment)
- `crates/argo-agent-runner/src/turn_helpers_truth_boundary.rs` (NEW) — helper module: refusal-variant picker, audit-payload builders, gate-call wrapper
- `crates/argo-agent-runner/src/turn_helpers_truth_boundary_tests.rs` (NEW)
- `crates/argo-agent-runner/src/lib.rs` (wire the new module(s))
- `crates/argo-core/src/agent.rs` (add 3 new fields to `AgentDeps` struct; update `Debug` impl if needed)
- `crates/argo-audit/src/event_kind.rs` (3 new `AuditKind` variant declarations + rustdoc)
- `crates/argo-audit/src/event.rs` (3 new `label` + `from_label` + `all` match arms / array entries)
- `crates/argo-capability/src/lib.rs` (re-export `RefusalTemplateRegistry` if newly added; do NOT touch `truth_boundary.rs`)
- `crates/argo-capability/src/refusal_registry.rs` (NEW, optional) — if a registry type is needed and doesn't already exist; keep it outside `truth_boundary.rs`
- All callsites of `AgentDeps::new` / struct-literal constructions across the workspace will need the new fields. Expected places: `argo-cli` agent boot, gateway run-routes, tests. Use `Default` or a builder if the ripple is large.

## Files you must NOT touch

- `crates/argo-capability/src/truth_boundary.rs` — P-01 owns it; the free function `evaluate_claim` is consumed via its public path.
- Anything in `crates/argo-skill/` — P-04 is the consumer that produces declared classes.
- The audit-write transport (`crates/argo-audit/src/sink.rs` chain-hash code, `sink_chain.rs`, etc.) — only the variant additions in `event.rs` / `event_kind.rs`. No schema migration; the rows slot into the existing `audit_record` table via `payload_json`.

## Acceptance criteria

- [ ] Every candidate response passing through `run_turn_with_tools` hits the gate exactly once when `outcome.tool_calls.is_empty()`.
- [ ] `Allow` path produces a `ClaimAllowed` audit row + propagates response unchanged.
- [ ] `Forbid` path swaps in a refusal variant and emits `ClaimBlocked`.
- [ ] `Escalate` path enqueues an approval and emits `ClaimEscalated`.
- [ ] Tenant scoping: cross-tenant negative test passes.
- [ ] Per-channel scoping: same claim class can be `Allow` on one channel and `Forbid` on another.
- [ ] `cargo test -p argo-agent-runner truth_boundary` runs ≥8 tests, all pass.
- [ ] `cargo test -p argo-audit` passes (no regressions from the 3 new variants).
- [ ] `cargo test -p argo-core` passes (no regressions from the new `AgentDeps` fields).
- [ ] No file >500 LOC; no fn >80 LOC; no `unwrap`/`expect` outside `#[cfg(test)]`.
- [ ] `audit-sink-tenant-guard` CI lint still passes.
- [ ] `bash scripts/check-brief-freshness.sh pitch-truth-boundary-runtime-enforcement` exits 0.

## Verification

```bash
CRATES="-p argo-agent-runner -p argo-audit -p argo-core -p argo-capability"
cargo clippy $CRATES --all-targets -- -D warnings 2>&1 | tail -30
cargo test $CRATES 2>&1 | tail -100
cargo test -p argo-agent-runner truth_boundary 2>&1 | tail -40
bash scripts/check-migration-slots.sh
bash scripts/check-brief-freshness.sh pitch-truth-boundary-runtime-enforcement
```

Integration smoke (manual, optional): seed a session with `skycot-public-pitch.toml` active, push a candidate response containing the word "valuation", confirm the audit chain shows a `ClaimBlocked` row and the delivered text is a refusal variant.

## Cross-references

- **Blocked by**: `pitch-truth-boundary-capability-schema` (P-01) — MERGED #2214.
- **Unblocks**: `pitch-agent-worker-template` (P-05) — manifest names the active policy; `pitch-skycot-web-embed-channel` (P-06) — registers as `ChannelKind::WebEmbed`; `pitch-meeting-orchestrator-zoom-channel` (P-07) — registers as `ChannelKind::LiveMeeting`; `pitch-canonical-qa-skill` (P-04) becomes the first `declared_classes` producer.
- **Audit page (B3 design)** consumes the new event variants — see `web-port-audit-b3-etherscan.md`.
- **Memory**: `project_pitch_agent_v1_demo.md`.
- **Storyboard**: [`docs/skycot/demos/pitch-agent-v1-storyboard.html`](../../skycot/demos/pitch-agent-v1-storyboard.html) — Scenes 2b/2c/3b/4b are the user-facing proof.
- **Refresh provenance**: this brief was BLOCKED 2026-05-25T07:22Z for drift; refreshed by `fix-pitch-truth-boundary-runtime-enforcement-brief-refresh` (meta-brief) against `ac7095d0c`.
