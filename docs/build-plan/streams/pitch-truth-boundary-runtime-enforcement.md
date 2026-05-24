# pitch-truth-boundary-runtime-enforcement

**Owner**: claude.
**Authored against**: origin/main `73ac6e06c` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh pitch-truth-boundary-runtime-enforcement`.
**Blocked by**: `pitch-truth-boundary-capability-schema` (P-01) MUST be MERGED before this is claimed.

🔴 **P0 v1.0 ship-blocker · Track P (Pitch Agent) · brief P-02 of 8**

Wires the `CapabilityEngine::evaluate_claim` evaluator (defined in P-01) into the agent runtime so that every utterance an agent emits passes through the truth-boundary gate before it leaves the process. This is the brief that makes the capability layer *enforced* rather than *defined*.

## Why this brief exists

P-01 adds the types; this brief adds the call site. Without P-02, the storyboard's three refusal beats (Scenes 2b/2c/4b) cannot happen — the agent would still hallucinate a valuation because no code path is checking. This brief inserts the check into `argo-agent-runner`'s turn loop at the single exit point where the agent's response becomes visible to the channel.

The operator-locked invariant: the gate is in the *runtime*, not in the *prompt*. A prompt can be talked out of; runtime enforcement cannot.

## Source of truth

- **P-01 brief**: `pitch-truth-boundary-capability-schema.md` — the types this consumes
- **Existing code**: `crates/argo-agent-runner/src/turn_helpers.rs` — the turn-loop helpers (LayeredLookup is wired here, per memory `project_v1_strategic_decisions_2026_05_14`)
- **Memory**: `project_pitch_agent_v1_demo.md` — "capability gate covers both paths, audit-logged"
- **Storyboard**: Scene 2b (audit panel slides in showing the block) — proves the audit emission is part of this brief

## Scope

### 1. Add the gate to the turn-helpers exit point

Locate the single function in `argo-agent-runner/src/turn_helpers.rs` (or sibling module) that finalizes an agent response into something a channel adapter consumes. Insert a synchronous gate call between "response candidate produced" and "response committed to channel."

Sketch (exact location varies — read the file first):

```rust
// In the turn loop, after producing a candidate response and before commit:
let candidate = ClaimCandidate {
    text: response.text.clone(),
    declared_classes: response.declared_claim_classes.clone(),
};

let eval = deps.capability_engine.evaluate_claim(
    &deps.active_truth_boundary_policy_id,
    deps.active_channel_kind,
    &candidate,
);

let final_response = match eval.disposition {
    ClaimDisposition::Allow => {
        deps.audit_sink.emit(AuditEvent::ClaimAllowed { provenance: eval.provenance, .. }).await?;
        response
    }
    ClaimDisposition::Forbid { refusal_template_id } => {
        deps.audit_sink.emit(AuditEvent::ClaimBlocked {
            matched_class: eval.matched_class,
            policy_id: deps.active_truth_boundary_policy_id.clone(),
            channel: deps.active_channel_kind,
        }).await?;
        let refusal = pick_refusal_variant(&deps.refusal_templates, &refusal_template_id)?;
        AgentResponse::refusal(refusal)
    }
    ClaimDisposition::Escalate => {
        deps.audit_sink.emit(AuditEvent::ClaimEscalated { .. }).await?;
        deps.approval_queue.enqueue(/* approval payload */).await?;
        AgentResponse::pending_approval()
    }
};
```

### 2. Wire `AgentDeps`

`AgentDeps` is the only injection point per the core invariants. Add the three new fields:

- `capability_engine: Arc<CapabilityEngine>` (likely already present — verify)
- `active_truth_boundary_policy_id: String` (per-session, resolved at session boot from the worker template manifest)
- `active_channel_kind: ChannelKind` (per-session, from the channel adapter that started the session)
- `refusal_templates: Arc<RefusalTemplateRegistry>` (loaded at engine boot)

If any of these can be derived from existing fields, prefer derivation over duplication. Audit-sink + approval-queue are already in `AgentDeps` (do not add).

### 3. Refusal-variant selection

`pick_refusal_variant(&registry, &id) -> Result<String>` — randomized variant pick (stable seed per session for testing). ~15 LOC. Lives in `argo-capability::truth_boundary` (the registry side) with a thin caller helper in `argo-agent-runner`.

### 4. Audit events

Three new audit-event variants (extend the existing `AuditEvent` enum in `argo-audit`):

- `AuditEvent::ClaimAllowed { provenance, matched_class, .. }`
- `AuditEvent::ClaimBlocked { matched_class, policy_id, channel, candidate_excerpt, .. }`
- `AuditEvent::ClaimEscalated { matched_class, policy_id, channel, approval_id, .. }`

Each carries the tenant_id (per the multi-tenant invariant). Audit sink already enforces tenant scoping (per `audit-sink-tenant-guard` CI lint).

### 5. Provenance propagation

The gate's evaluation includes a `ClaimProvenance` struct (from P-01). Persist it as part of the audit row so the audit page (B3 design) can show "where this answer came from" — canonical-qa hit, approved-memory pull, or generative.

### 6. Channel-context resolution

The runtime needs to know which `ChannelKind` is active. Add a one-line accessor on the existing `argo-channels` channel-adapter trait:

```rust
pub trait ChannelAdapter {
    // existing methods...
    fn kind(&self) -> ChannelKind;  // new
}
```

Default impls on existing adapters (Slack/email/CLI/etc.) return `ChannelKind::Chat`. P-06 and P-07 will add `WebEmbed` and `LiveMeeting` impls.

### 7. Tests

`crates/argo-agent-runner/src/turn_helpers_truth_boundary_tests.rs`:

- An allowed claim passes through with an `AuditEvent::ClaimAllowed` row
- A forbidden claim is replaced with a refusal variant and emits `AuditEvent::ClaimBlocked`
- An escalated claim queues an approval and emits `AuditEvent::ClaimEscalated`
- Cross-tenant: tenant A's policy does not gate tenant B's session
- Per-channel: same claim class is `Allow` on `Rehearsal` and `Forbid` on `WebEmbed`
- Provenance: a candidate from canonical-qa carries `ProvenanceSource::CanonicalQa` through to the audit row

Aim for ≥8 test cases.

## Files you may touch

- `crates/argo-agent-runner/src/turn_helpers.rs` (insert the gate)
- `crates/argo-agent-runner/src/agent_deps.rs` (add fields if structure exists; else thread via the existing dep struct)
- `crates/argo-agent-runner/src/turn_helpers_truth_boundary_tests.rs` (new)
- `crates/argo-audit/src/events.rs` (add three variants)
- `crates/argo-channels/src/lib.rs` (add `kind()` to the trait + default impls)

## Files you must NOT touch

- `crates/argo-capability/src/truth_boundary.rs` — P-01 owns it
- Anything in `argo-skill/` — P-04 is the consumer that produces declared classes
- The audit-write side beyond adding the three new event variants. No schema changes to the audit table — these slot into the existing payload column

## Acceptance criteria

- [ ] Every candidate response passing through the turn loop hits the gate exactly once
- [ ] `Allow` path produces audit row + propagates response unchanged
- [ ] `Forbid` path swaps in a refusal variant and emits `ClaimBlocked` audit row
- [ ] `Escalate` path enqueues an approval and emits `ClaimEscalated` audit row
- [ ] Tenant scoping: cross-tenant negative test passes
- [ ] Per-channel scoping: same claim class can be `Allow` on one channel and `Forbid` on another
- [ ] `cargo test -p argo-agent-runner truth_boundary` passes ≥8 tests
- [ ] `cargo test -p argo-audit` passes (no regressions from new variants)
- [ ] No file >500 LOC; no fn >80 LOC; no `unwrap`/`expect` outside `#[cfg(test)]`
- [ ] Audit-sink-tenant-guard CI lint still passes

## Verification

```bash
cargo build -p argo-agent-runner -p argo-audit -p argo-channels
cargo test -p argo-agent-runner truth_boundary
cargo test -p argo-audit
bash scripts/check-migration-slots.sh
bash scripts/check-brief-freshness.sh pitch-truth-boundary-runtime-enforcement
```

Integration smoke (manual): seed a session with `skycot-public-pitch.toml` active, push a candidate response containing the word "valuation," confirm the audit chain shows a `ClaimBlocked` row with a refusal variant in the final response.

## Cross-references

- **Blocked by**: `pitch-truth-boundary-capability-schema` (P-01)
- **Unblocks**: `pitch-agent-worker-template` (P-05) — the manifest names the active policy; `pitch-skycot-web-embed-channel` (P-06) — registers as `ChannelKind::WebEmbed`; `pitch-meeting-orchestrator-zoom-channel` (P-07) — registers as `ChannelKind::LiveMeeting`
- **Audit page (B3 design)** consumes the new event variants — see `web-port-audit-b3-etherscan.md`
- **Memory**: `project_pitch_agent_v1_demo.md`
- **Storyboard**: [`docs/skycot/demos/pitch-agent-v1-storyboard.html`](../../skycot/demos/pitch-agent-v1-storyboard.html) — Scenes 2b/2c/3b/4b are the user-facing proof of this brief
