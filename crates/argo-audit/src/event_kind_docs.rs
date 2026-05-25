//! Long-form notes for verbose [`crate::AuditKind`] variants.
//!
//! `event_kind.rs` deliberately stays compact because new audit kinds
//! are added frequently and the swarm enforces a 500-LOC file ceiling.
//! Keep short rustdoc on each enum variant there; use this module for
//! details that explain payload shape, emission timing, and forensic
//! pairing rules.
//!
//! Approval flow notes:
//! - `ApprovalAsked` is emitted after an outbound channel `send()`
//!   succeeds. Failed sends do not write phantom ask rows. A fallback
//!   chain that lands on one channel writes one row; fan-out writes one
//!   row per successful delivery.
//! - `ApprovalReplied` is emitted after inbound verification and
//!   payload decoding, before the gate's approve/reject transition.
//!   First reply wins; late replies hit the existing log/discard path.
//! - `ApprovalDeclined` records a gate-side decline decision. It is
//!   distinct from `ApprovalRevoked` (undo a prior grant) and from an
//!   inbound `ApprovalReplied` reject reply.
//!
//! Runtime and gateway notes:
//! - `HttpRequest` is recorded after route execution so payloads can
//!   include response status, duration, method, query-stripped path,
//!   and resolved principal. Probe routes stay out of the chain.
//! - `WorkerOomRestart` is a whole-process supervisor event, distinct
//!   from per-turn `Halt` and per-action executor retry strategies.
//! - `RuntimeContextCompressed` is one row per compression pass, with
//!   token counts matching `argo_runtime::context_compress`.
//! - `SecurityKeyRotated` preserves the historical
//!   `security.key_rotated` label so old chain hashes remain valid.
//!
//! Product-surface notes:
//! - Workflow rollback is additive: rollback copies a prior definition
//!   into a new version and pairs with `WorkflowHeadChanged`.
//! - Workflow-step and DAG-step variants are separate. The former
//!   covers vertical-workflow skill-invocation boundaries; the latter
//!   covers production runner dispatch success/failure.
//! - Workplace-knowledge lifecycle variants make wiki mutations
//!   filterable without parsing generic `HttpRequest` payloads.
//! - `GenesisFallbackUsed` pairs with `WorkerHired`; a hire without
//!   the fallback row means the gateway Genesis draft served cleanly.
//!
//! Duty and handoff notes:
//! - `WorkerDutyFired` and `WorkerDutyFailed` are mutually exclusive
//!   outcomes for a fired duty. The scheduler keeps ticking after a
//!   failed runner call, so the failure row is the loud signal.
//! - `HandoffCrossTenantAttempt` preserves the substrate's
//!   byte-identical outward error while appending a victim-tenant audit
//!   row when the session exists across tenant boundaries.
