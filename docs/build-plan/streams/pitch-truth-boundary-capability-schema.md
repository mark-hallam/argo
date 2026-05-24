# pitch-truth-boundary-capability-schema

**Owner**: claude.
**Authored against**: origin/main `73ac6e06c` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh pitch-truth-boundary-capability-schema`.

🔴 **P0 v1.0 ship-blocker · Track P (Pitch Agent) · brief P-01 of 8**

The foundation primitive for the Pitch Agent (v1.0 demo headline, see [`docs/skycot/demos/pitch-agent-v1-storyboard.html`](../../skycot/demos/pitch-agent-v1-storyboard.html)). Extends `argo-capability` with claim-level access control so the runtime can enforce "what an agent is allowed to say" the same way it enforces "what an agent is allowed to do." Every utterance the Pitch Agent produces must pass through this gate before it leaves the runtime.

This brief MUST land before P-02 (runtime enforcement) and P-05 (worker template) — both depend on the types defined here.

## Why this brief exists

The v1.0 demo storyboard locks three truth-boundary refusal beats (Scene 2b valuation, Scene 2c ARR-rehearsal, Scene 4b ARR-on-stage). In every beat, the agent refuses to make a claim *because the capability layer prevented it*, not because the prompt told it to. Prompt-level guardrails leak; capability-level guardrails are auditable and enforceable.

The operator-locked memory `project_pitch_agent_v1_demo.md` calls this out explicitly: "truth-boundary is the brand/legal risk. Capability-enforced forbidden claims (not prompt-instructed) is the load-bearing investment."

`argo-capability` today scopes *tool access* (`Capability { tool: "send_email" }`-style). This brief adds a parallel axis: *claim access* — a policy that gates what classes of statement an agent may produce, per channel, per tenant.

## Source of truth

- **Storyboard**: [`docs/skycot/demos/pitch-agent-v1-storyboard.html`](../../skycot/demos/pitch-agent-v1-storyboard.html) — Scenes 2b, 2c, 3b, 4b each show this primitive at work
- **Memory**: `~/.claude/projects/-Users-operator-code-argo/memory/project_pitch_agent_v1_demo.md` (locked 2026-05-24)
- **Existing crate**: `crates/argo-capability/src/lib.rs` — `Capability`, `CapabilityEngine`, `CapabilityRole` are the existing surfaces to extend
- **Core platform invariant**: capability-based security is invariant #3 — extending the existing crate is correct; forking it would violate the invariant

## Scope

### 1. New module: `crates/argo-capability/src/truth_boundary.rs`

Add a new module (do NOT add a new crate). This sits alongside the existing tool-access capability machinery.

Add the following public types:

```rust
/// A policy that gates what classes of claim an agent may produce.
/// Identified by `id` (e.g. "skycot-public-pitch", "skycot-private-rehearsal").
pub struct TruthBoundaryPolicy {
    pub id: String,
    pub tenant_id: TenantId,
    pub claim_classes: Vec<ClaimClass>,
}

/// A single claim class — a category of statement the agent might produce.
/// Examples: "revenue", "valuation", "customer-count", "roadmap-promise".
pub struct ClaimClass {
    pub name: String,
    pub disposition: ClaimDisposition,
    pub channels: ChannelScope,
    pub escalate_to: Option<EscalationTarget>,
}

/// What the runtime should do when an agent attempts to make a claim in this class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimDisposition {
    /// Allowed; pass through with audit.
    Allow,
    /// Forbidden; block with a templated refusal + audit.
    Forbid { refusal_template_id: String },
    /// Escalate to a human; queue an approval, block until resolved.
    Escalate,
}

/// Per-channel scoping — a claim may be Allow on private channels but Forbid on public.
#[derive(Debug, Clone)]
pub enum ChannelScope {
    AllChannels,
    Only(Vec<ChannelKind>),
    Except(Vec<ChannelKind>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ChannelKind {
    Rehearsal,       // private operator-only voice loop
    WebEmbed,        // public web widget (skycot.com/pitch)
    LiveMeeting,     // joined Zoom/Meet
    Chat,            // text-only channel (Slack/etc.)
}

/// Where escalations route. Mirrors existing approval routing.
pub struct EscalationTarget {
    pub kind: EscalationKind, // Approver | Human | Webhook
    pub address: String,      // user_id, email, URL — discriminated by `kind`
}

/// Result of evaluating a candidate utterance against the policy.
pub struct ClaimEvaluation {
    pub disposition: ClaimDisposition,
    pub matched_class: Option<String>,  // claim_class.name if matched
    pub confidence: f32,                // 0.0..=1.0
    pub provenance: ClaimProvenance,
}

pub struct ClaimProvenance {
    pub source: ProvenanceSource,
    pub source_id: String,
    pub source_excerpt: String,
}

pub enum ProvenanceSource {
    CanonicalQa,     // matched a pre-warmed answer (see P-04)
    ApprovedMemory,  // pulled from approved-facts memory
    Generative,      // produced by the model
    EscalationStub,  // produced by the escalation flow
}
```

### 2. Evaluator API

Add a single evaluator function — keep the API surface narrow:

```rust
impl CapabilityEngine {
    /// Evaluate a candidate utterance against the active truth-boundary policy.
    /// Returns the disposition + provenance. The runtime decides what to do with it.
    ///
    /// `policy_id`: looked up via the engine's existing policy resolution.
    /// `channel`: which channel the utterance is destined for.
    /// `candidate`: the proposed response text (plus structured claim metadata if available).
    pub fn evaluate_claim(
        &self,
        policy_id: &str,
        channel: ChannelKind,
        candidate: &ClaimCandidate,
    ) -> ClaimEvaluation { /* ... */ }
}

pub struct ClaimCandidate {
    pub text: String,
    /// Optional structured signals from the producer (e.g. canonical-qa skill
    /// can declare "this answer is in class 'comparison'" to short-circuit
    /// classification).
    pub declared_classes: Vec<String>,
}
```

The evaluator is a pure function over policy + candidate. No I/O. No logging from inside — logging is the caller's job (P-02 wires audit).

### 3. Policy storage + loading

Policies are TOML files loaded at engine boot, same pattern as `default_roles()`. Add:

- `crates/argo-capability/src/truth_boundary_policies.rs` — loader, validator
- `policies/truth-boundary/skycot-public-pitch.toml` (example policy at repo root, NOT in `~/.argo/` — examples live in the repo so they can be reviewed)
- `policies/truth-boundary/skycot-private-rehearsal.toml`

Schema is straightforward TOML:

```toml
id = "skycot-public-pitch"
tenant_id = "skycot"

[[claim_classes]]
name = "revenue"
disposition = "Forbid"
refusal_template_id = "defer-to-mark-financial"
channels = "AllChannels"

[[claim_classes]]
name = "customer-count"
disposition = "Forbid"
refusal_template_id = "defer-to-public-customers-page"
channels = "Only:WebEmbed,LiveMeeting"

[[claim_classes]]
name = "unannounced-roadmap"
disposition = "Escalate"
channels = "AllChannels"

[[claim_classes.escalate_to]]
kind = "Approver"
address = "founder@example.com"
```

### 4. Refusal-template store

`ClaimDisposition::Forbid { refusal_template_id }` references a refusal-template ID. Store these in a sibling registry:

```rust
pub struct RefusalTemplate {
    pub id: String,
    pub variants: Vec<String>,  // multiple wordings; runtime picks one
}

pub fn load_refusal_templates(workspace_root: &Path) -> Vec<RefusalTemplate>;
```

TOML schema:

```toml
id = "defer-to-mark-financial"
variants = [
  "I'm not authorized to share that publicly. Mark, do you want to take it?",
  "That number's not public — happy to put you on a call with Mark for the specifics.",
  "I'll defer to Mark on financials.",
]
```

The runtime (P-02) selects a variant; this brief just defines the schema.

### 5. Tests

`crates/argo-capability/src/truth_boundary_tests.rs` — unit tests covering:

- `evaluate_claim` returns `Allow` for unmatched classes
- `evaluate_claim` returns `Forbid` for matched forbidden class on in-scope channel
- `evaluate_claim` returns `Allow` for matched forbidden class on out-of-scope channel (per `ChannelScope::Except`)
- Policy loading rejects malformed TOML with a useful error
- Refusal-template loading rejects empty `variants` arrays
- Multi-tenant: policies are scoped by `tenant_id`; querying with the wrong tenant returns the default (no-match) policy

Aim for ≥10 test cases. Cross-tenant negative test is mandatory (per the existing `argo-capability` test pattern).

## Files you may touch

- `crates/argo-capability/src/lib.rs` (re-export new types)
- `crates/argo-capability/src/truth_boundary.rs` (new)
- `crates/argo-capability/src/truth_boundary_policies.rs` (new)
- `crates/argo-capability/src/truth_boundary_tests.rs` (new)
- `crates/argo-capability/Cargo.toml` (add `toml`/`serde` deps if not present)
- `policies/truth-boundary/skycot-public-pitch.toml` (new)
- `policies/truth-boundary/skycot-private-rehearsal.toml` (new)

## Files you must NOT touch

- Anything outside `argo-capability` and `policies/truth-boundary/`. P-02 wires this into `argo-agent-runner`; don't pre-empt that.
- The existing tool-capability machinery (`Capability`, `CapabilityEngine` impl bodies, `default_roles`). Add a new module; don't refactor.

## Acceptance criteria

- [ ] `argo-capability` builds with the new module
- [ ] `cargo test -p argo-capability truth_boundary` passes ≥10 tests
- [ ] Cross-tenant negative test passes (policy lookup for wrong tenant returns no-match)
- [ ] Two example policies (`skycot-public-pitch.toml`, `skycot-private-rehearsal.toml`) load cleanly from the workspace root
- [ ] Refusal-template loader rejects empty `variants` with a clear error
- [ ] `evaluate_claim` is pure (no I/O); audit emission is deferred to P-02
- [ ] Public surface is exported from `crates/argo-capability/src/lib.rs`
- [ ] No file >500 LOC; no fn >80 LOC; no `unwrap`/`expect` outside `#[cfg(test)]`
- [ ] Rustdoc on every public type/fn with at least one example (per core-platform invariant)

## Verification

```bash
cargo build -p argo-capability
cargo test -p argo-capability truth_boundary
cargo doc -p argo-capability --no-deps
bash scripts/check-brief-freshness.sh pitch-truth-boundary-capability-schema
```

Sanity-check the policy files parse:

```bash
cargo run --example load_truth_boundary_policies -- policies/truth-boundary/skycot-public-pitch.toml
```

(Add the example binary if it doesn't exist; ~20 LOC.)

## Cross-references

- **Storyboard**: [`docs/skycot/demos/pitch-agent-v1-storyboard.html`](../../skycot/demos/pitch-agent-v1-storyboard.html) — Scenes 2b/2c/3b/4b show the user-facing behavior this primitive enables
- **Next brief**: `pitch-truth-boundary-runtime-enforcement` (P-02) — wires this evaluator into the turn loop
- **Used by**: `pitch-canonical-qa-skill` (P-04) — declares structured `declared_classes` to inform evaluation; `pitch-agent-worker-template` (P-05) — manifest names the active policy ID
- **Memory**: `project_pitch_agent_v1_demo.md` — scope; `project_v1_strategic_decisions_2026_05_14.md` — type canonicalization preference (extend `argo-capability`, don't fork)
- **Drift rule**: pre-flight grep run 2026-05-24 against `crates/` — no existing `TruthBoundaryPolicy`, `ClaimDisposition`, `ClaimEvaluation`, `RefusalTemplate` surfaces; safe to author
