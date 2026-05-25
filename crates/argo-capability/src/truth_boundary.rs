//! Truth-boundary policy primitives — the capability layer for *what an
//! agent is allowed to say*, parallel to the existing tool-dispatch layer
//! for *what an agent is allowed to do*.
//!
//! See `docs/build-plan/streams/pitch-truth-boundary-capability-schema.md`
//! (brief P-01 of the Pitch Agent track). This module defines pure data
//! types + a pure evaluator. Audit emission, refusal-variant selection,
//! and runtime wiring live in P-02 (`argo-agent-runner`).
//!
//! ```
//! use argo_capability::truth_boundary::{
//!     ChannelKind, ChannelScope, ClaimCandidate, ClaimClass, ClaimDisposition,
//!     TruthBoundaryPolicy, evaluate_claim,
//! };
//! use argo_types::TenantId;
//!
//! let policy = TruthBoundaryPolicy {
//!     id: "skycot-public-pitch".into(),
//!     tenant_id: TenantId::new(),
//!     claim_classes: vec![ClaimClass {
//!         name: "revenue".into(),
//!         disposition: ClaimDisposition::Forbid {
//!             refusal_template_id: "defer-to-mark-financial".into(),
//!         },
//!         channels: ChannelScope::AllChannels,
//!         escalate_to: None,
//!     }],
//! };
//! let candidate = ClaimCandidate {
//!     text: "Our ARR is $4.2M.".into(),
//!     declared_classes: vec!["revenue".into()],
//! };
//! let result = evaluate_claim(&policy, ChannelKind::WebEmbed, &candidate);
//! assert!(matches!(result.disposition, ClaimDisposition::Forbid { .. }));
//! ```

use argo_types::TenantId;
use serde::{Deserialize, Serialize};

/// A policy that gates what classes of claim an agent may produce.
///
/// Identified by `id` (e.g. `"skycot-public-pitch"`,
/// `"skycot-private-rehearsal"`). Scoped to a tenant — cross-tenant
/// lookups return no match (the runtime treats this as Allow-by-default,
/// see P-02). Policies are loaded from TOML at engine boot via
/// [`crate::truth_boundary_policies::load_policy`].
///
/// ```
/// use argo_capability::truth_boundary::{ClaimClass, ClaimDisposition,
///     ChannelScope, TruthBoundaryPolicy};
/// use argo_types::TenantId;
///
/// let p = TruthBoundaryPolicy {
///     id: "skycot-public-pitch".into(),
///     tenant_id: TenantId::new(),
///     claim_classes: vec![ClaimClass {
///         name: "valuation".into(),
///         disposition: ClaimDisposition::Forbid {
///             refusal_template_id: "defer-to-mark-financial".into(),
///         },
///         channels: ChannelScope::AllChannels,
///         escalate_to: None,
///     }],
/// };
/// assert_eq!(p.claim_classes.len(), 1);
/// ```
#[derive(Debug, Clone)]
pub struct TruthBoundaryPolicy {
    /// Stable identifier (matches the `policy_id` field on agent manifests).
    pub id: String,
    /// Tenant this policy belongs to. Cross-tenant lookups are denied.
    pub tenant_id: TenantId,
    /// Ordered list of claim classes. First-match wins during evaluation.
    pub claim_classes: Vec<ClaimClass>,
}

/// A single claim class — a category of statement the agent might
/// produce. Examples: `"revenue"`, `"valuation"`, `"customer-count"`,
/// `"unannounced-roadmap"`.
///
/// ```
/// use argo_capability::truth_boundary::{ChannelScope, ClaimClass,
///     ClaimDisposition};
///
/// let c = ClaimClass {
///     name: "customer-count".into(),
///     disposition: ClaimDisposition::Forbid {
///         refusal_template_id: "defer-to-public-customers-page".into(),
///     },
///     channels: ChannelScope::AllChannels,
///     escalate_to: None,
/// };
/// assert_eq!(c.name, "customer-count");
/// ```
#[derive(Debug, Clone)]
pub struct ClaimClass {
    /// Stable class name (free-form; producers and evaluators agree by string).
    pub name: String,
    /// What the runtime should do when an agent attempts a claim in this class.
    pub disposition: ClaimDisposition,
    /// Channels this class applies to. `Only` is allow-list, `Except` is deny-list.
    pub channels: ChannelScope,
    /// Where to route an `Escalate` disposition. Ignored for `Allow`/`Forbid`.
    pub escalate_to: Option<EscalationTarget>,
}

/// What the runtime should do when an agent attempts a claim in a given class.
///
/// `Allow` is the implicit default for any class not listed in a policy —
/// callers don't need to enumerate Allow rules. `Forbid` carries the
/// refusal-template id used to render the user-facing refusal copy
/// (selection of a variant happens at the runtime layer, P-02).
/// `Escalate` queues an approval and blocks the response until resolved.
///
/// ```
/// use argo_capability::truth_boundary::ClaimDisposition;
///
/// let d = ClaimDisposition::Forbid {
///     refusal_template_id: "defer-to-mark-financial".into(),
/// };
/// match d {
///     ClaimDisposition::Forbid { refusal_template_id } => {
///         assert_eq!(refusal_template_id, "defer-to-mark-financial");
///     }
///     _ => unreachable!(),
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimDisposition {
    /// Allowed; pass through (P-02 still emits an audit event).
    Allow,
    /// Forbidden; the runtime renders a refusal from `refusal_template_id`.
    Forbid {
        /// Look-up key into the refusal-template registry (see
        /// [`crate::truth_boundary::RefusalTemplate`]).
        refusal_template_id: String,
    },
    /// Escalate to a human; queue an approval, block until resolved.
    Escalate,
}

/// Per-channel scoping. A claim class may apply on some channels but
/// not others — e.g. revenue numbers may be Allow on the private
/// rehearsal channel but Forbid on the public web embed.
///
/// ```
/// use argo_capability::truth_boundary::{ChannelKind, ChannelScope};
///
/// let s = ChannelScope::Only(vec![ChannelKind::WebEmbed]);
/// match s {
///     ChannelScope::Only(chans) => assert_eq!(chans.len(), 1),
///     _ => unreachable!(),
/// }
/// ```
#[derive(Debug, Clone)]
pub enum ChannelScope {
    /// Apply to all channels.
    AllChannels,
    /// Apply only on the listed channels.
    Only(Vec<ChannelKind>),
    /// Apply on every channel EXCEPT the listed ones.
    Except(Vec<ChannelKind>),
}

impl ChannelScope {
    /// Returns true if the scope covers `channel` — i.e. the
    /// surrounding [`ClaimClass`] should be evaluated when an
    /// utterance is destined for that channel.
    ///
    /// ```
    /// use argo_capability::truth_boundary::{ChannelKind, ChannelScope};
    ///
    /// let s = ChannelScope::Except(vec![ChannelKind::Rehearsal]);
    /// assert!(s.covers(ChannelKind::WebEmbed));
    /// assert!(!s.covers(ChannelKind::Rehearsal));
    /// ```
    #[must_use]
    pub fn covers(&self, channel: ChannelKind) -> bool {
        match self {
            ChannelScope::AllChannels => true,
            ChannelScope::Only(list) => list.contains(&channel),
            ChannelScope::Except(list) => !list.contains(&channel),
        }
    }
}

/// The set of channels the Pitch Agent can reach. P-02 will extend
/// `argo-channels` with a corresponding adapter trait; the type lives
/// here because policy authoring is the originating constraint.
///
/// ```
/// use argo_capability::truth_boundary::ChannelKind;
///
/// assert_ne!(ChannelKind::WebEmbed, ChannelKind::Rehearsal);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChannelKind {
    /// Private operator-only voice loop (no public surface).
    Rehearsal,
    /// Public web widget (e.g. skycot.com/pitch).
    WebEmbed,
    /// Joined live meeting (Zoom / Meet).
    LiveMeeting,
    /// Text-only channel (Slack, iMessage, email).
    Chat,
}

/// Where an `Escalate` disposition routes. Mirrors the address-shape
/// of the existing approval routing — the `address` field is
/// discriminated by `kind` (user_id, email, URL).
///
/// ```
/// use argo_capability::truth_boundary::{EscalationKind, EscalationTarget};
///
/// let t = EscalationTarget {
///     kind: EscalationKind::Approver,
///     address: "founder@example.com".into(),
/// };
/// assert_eq!(t.kind, EscalationKind::Approver);
/// ```
#[derive(Debug, Clone)]
pub struct EscalationTarget {
    /// How `address` is interpreted (Approver: user_id, Human: email,
    /// Webhook: URL).
    pub kind: EscalationKind,
    /// Channel-specific address — semantics depend on `kind`.
    pub address: String,
}

/// How an [`EscalationTarget`]'s `address` should be interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscalationKind {
    /// `address` is an Argo principal/user id; routes via the
    /// approvals subsystem.
    Approver,
    /// `address` is an email; routes via the email channel.
    Human,
    /// `address` is a webhook URL; the runtime POSTs the escalation
    /// payload there.
    Webhook,
}

/// A candidate utterance to evaluate. The `text` is the agent's
/// proposed response; `declared_classes` is a list of claim-class
/// names the producer asserts apply (canonical-Q&A skill, P-04, sets
/// this to short-circuit semantic classification). The classifier in
/// P-02 may add additional matches; this brief's evaluator treats
/// `declared_classes` as the only signal.
///
/// ```
/// use argo_capability::truth_boundary::ClaimCandidate;
///
/// let c = ClaimCandidate {
///     text: "Our ARR is $4.2M".into(),
///     declared_classes: vec!["revenue".into()],
/// };
/// assert_eq!(c.declared_classes.len(), 1);
/// ```
#[derive(Debug, Clone)]
pub struct ClaimCandidate {
    /// The proposed response text.
    pub text: String,
    /// Structured class signals from the producer. Empty if the
    /// classifier needs to infer from text (P-02 wires the inference;
    /// this module treats empty as "no match → Allow").
    pub declared_classes: Vec<String>,
}

/// Result of evaluating a candidate utterance against a policy.
///
/// The evaluator is pure — `disposition` is what the policy says
/// should happen, but the caller (P-02) decides how to act on it
/// (render a refusal, queue an escalation, emit audit, …).
///
/// ```
/// use argo_capability::truth_boundary::{ClaimDisposition, ClaimEvaluation,
///     ClaimProvenance, ProvenanceSource};
///
/// let e = ClaimEvaluation {
///     disposition: ClaimDisposition::Allow,
///     matched_class: None,
///     confidence: 1.0,
///     provenance: ClaimProvenance {
///         source: ProvenanceSource::Generative,
///         source_id: "turn-42".into(),
///         source_excerpt: String::new(),
///     },
/// };
/// assert_eq!(e.disposition, ClaimDisposition::Allow);
/// ```
#[derive(Debug, Clone)]
pub struct ClaimEvaluation {
    /// The policy's verdict for this candidate.
    pub disposition: ClaimDisposition,
    /// The claim-class name that matched (None if Allow-by-default).
    pub matched_class: Option<String>,
    /// Confidence in the match, 0.0..=1.0. Declared classes match at
    /// 1.0; inferred classes (P-02) will use the classifier's score.
    pub confidence: f32,
    /// Where the candidate came from. Captured so P-02 can audit
    /// "this answer is canonical vs generative" alongside the verdict.
    pub provenance: ClaimProvenance,
}

/// Provenance metadata for an evaluated candidate.
///
/// ```
/// use argo_capability::truth_boundary::{ClaimProvenance, ProvenanceSource};
///
/// let p = ClaimProvenance {
///     source: ProvenanceSource::CanonicalQa,
///     source_id: "qa-team-size-v1".into(),
///     source_excerpt: "Skycot team is 4 engineers".into(),
/// };
/// assert_eq!(p.source, ProvenanceSource::CanonicalQa);
/// ```
#[derive(Debug, Clone)]
pub struct ClaimProvenance {
    /// What kind of producer emitted this candidate.
    pub source: ProvenanceSource,
    /// Stable id for the source (canonical-Q&A entry id, memory id, turn id).
    pub source_id: String,
    /// Excerpt for audit visibility. Truncated by the caller as needed.
    pub source_excerpt: String,
}

/// The kind of producer that emitted a candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProvenanceSource {
    /// Matched a pre-warmed canonical Q&A entry (P-04).
    CanonicalQa,
    /// Pulled from approved-facts memory.
    ApprovedMemory,
    /// Produced by the model.
    Generative,
    /// Produced by the escalation flow (e.g. operator-typed answer).
    EscalationStub,
}

/// A user-facing refusal template — a registry entry referenced by
/// [`ClaimDisposition::Forbid::refusal_template_id`]. The runtime
/// (P-02) selects one of `variants` to render.
///
/// ```
/// use argo_capability::truth_boundary::RefusalTemplate;
///
/// let t = RefusalTemplate {
///     id: "defer-to-mark-financial".into(),
///     variants: vec!["I'll defer to Mark on financials.".into()],
/// };
/// assert!(!t.variants.is_empty());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefusalTemplate {
    /// Stable id (matches `refusal_template_id` on [`ClaimDisposition::Forbid`]).
    pub id: String,
    /// One or more wordings. The runtime picks at random / round-robin.
    /// At least one variant is required; the loader rejects empty arrays.
    pub variants: Vec<String>,
}

/// Pure evaluator — given a policy + channel + candidate, return the
/// policy's verdict.
///
/// The function is intentionally free-standing (not a `CapabilityEngine`
/// method): the engine is the policy-lookup integration point, but
/// evaluation itself has no I/O and benefits from being unit-testable
/// in isolation. P-02 will add `CapabilityEngine::evaluate_claim` as a
/// thin wrapper that resolves a `policy_id` to a [`TruthBoundaryPolicy`]
/// then calls this function.
///
/// Matching algorithm:
/// 1. For each declared class on the candidate, find the first
///    [`ClaimClass`] in the policy whose `name` matches AND whose
///    `channels` scope covers `channel`.
/// 2. If a match is found, return its disposition with `confidence = 1.0`
///    and `matched_class = Some(name)`.
/// 3. If no match, return `ClaimDisposition::Allow` with
///    `matched_class = None` and `confidence = 1.0` (the absence of
///    a forbidden-class match IS a positive Allow signal at the
///    policy level — semantic classification is P-02's job).
///
/// ```
/// use argo_capability::truth_boundary::{
///     ChannelKind, ChannelScope, ClaimCandidate, ClaimClass, ClaimDisposition,
///     TruthBoundaryPolicy, evaluate_claim,
/// };
/// use argo_types::TenantId;
///
/// let policy = TruthBoundaryPolicy {
///     id: "p1".into(),
///     tenant_id: TenantId::new(),
///     claim_classes: vec![ClaimClass {
///         name: "revenue".into(),
///         disposition: ClaimDisposition::Forbid {
///             refusal_template_id: "defer-to-mark-financial".into(),
///         },
///         channels: ChannelScope::AllChannels,
///         escalate_to: None,
///     }],
/// };
/// let r = evaluate_claim(
///     &policy,
///     ChannelKind::WebEmbed,
///     &ClaimCandidate {
///         text: "ARR is $4.2M".into(),
///         declared_classes: vec!["revenue".into()],
///     },
/// );
/// assert!(matches!(r.disposition, ClaimDisposition::Forbid { .. }));
/// assert_eq!(r.matched_class.as_deref(), Some("revenue"));
/// ```
#[must_use]
pub fn evaluate_claim(
    policy: &TruthBoundaryPolicy,
    channel: ChannelKind,
    candidate: &ClaimCandidate,
) -> ClaimEvaluation {
    for declared in &candidate.declared_classes {
        for class in &policy.claim_classes {
            if class.name == *declared && class.channels.covers(channel) {
                return ClaimEvaluation {
                    disposition: class.disposition.clone(),
                    matched_class: Some(class.name.clone()),
                    confidence: 1.0,
                    provenance: default_provenance(),
                };
            }
        }
    }
    ClaimEvaluation {
        disposition: ClaimDisposition::Allow,
        matched_class: None,
        confidence: 1.0,
        provenance: default_provenance(),
    }
}

fn default_provenance() -> ClaimProvenance {
    ClaimProvenance {
        source: ProvenanceSource::Generative,
        source_id: String::new(),
        source_excerpt: String::new(),
    }
}
