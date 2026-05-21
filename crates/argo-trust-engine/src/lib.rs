//! Periodic worker trust-grade recomputation
//! (`web-worker-trust-promotion-engine`).
//!
//! Before this crate landed, `worker.trust_grade` was set once at
//! hire and never changed. The [`/trust` UI](../../../../web/app/(app)/workers/[workerId]/trust/page.tsx)
//! described what would unlock the next rung but nothing in the
//! runtime ever advanced it. This crate is the missing engine.
//!
//! Five read-only signal streams are aggregated into a per-worker
//! score; the score is then mapped to a
//! [`argo_skill::TrustGrade`] with hysteresis (the promote
//! threshold sits above the demote threshold for each rung) so
//! scores hovering near a threshold do not flap the grade every
//! tick. The engine ratchets at most one rung per tick — a worker
//! with a great week climbs visibly in `/audit`, one rung at a
//! time, rather than mystery-jumping from `learned_unreviewed` to
//! `builtin`.
//!
//! Signal sources (none new — all already populated):
//!
//! | Signal | Source | Sign |
//! |--------|--------|------|
//! | Approval streak length    | `trust_streak_event` view             | + |
//! | Approval rate (30d)       | `audit_record` kind=`approval.granted` / `approval.replied` | ± |
//! | Escalation count (30d)    | `audit_record` kind=`escalation.required` | − |
//! | RBAC denial count (30d)   | `audit_record` kind=`rbac.deny`        | − |
//! | Cross-tenant quality avg  | `trust_observation`                    | + |
//!
//! Spawn the cron loop with [`spawn_trust_engine_cron`]; one-shot
//! recompute is available via [`recompute::recompute_worker`].
//!
//! ## Sensible deviations from the brief
//!
//! - The brief showed `sink.emit(...)` against an `&dyn AuditSink`
//!   trait object. The real [`argo_audit::AuditSink`] is a concrete
//!   type with a `.write(...)` API, so the recompute driver takes
//!   `&argo_audit::AuditSink` and calls `write` directly.
//! - The brief's queries filtered on `audit_record.action_type` but
//!   that denormalized column is sourced from `payload_json.action_type`
//!   (see [`argo_audit::filter_dims::extract_filter_dims`]) — it does
//!   NOT receive the kind label. The implemented queries filter on
//!   the `kind` column with values like `'approval.granted'`, which is
//!   what `AuditKind::ApprovalGranted.label()` actually persists.
//! - The brief listed `argo-trust-engine.workspace = true` for the
//!   CLI dep, but the rest of `argo-cli/Cargo.toml` uses path-form
//!   `argo-foo = { path = "../argo-foo" }`. Matched the surrounding
//!   convention.
//! - The brief's `argo_skill::TrustGrade` table claimed `Builtin = 4`
//!   (top) by enum index. The actual variant order is `Builtin = 0`
//!   (top) → `LearnedUnreviewed = 4` (bottom); `worker.trust_grade`
//!   u8 is INVERTED (0 = lowest, 4 = highest). The existing
//!   `routes_worker_trust_grade::grade_for_u8` already encodes the
//!   inversion. [`ratchet::grade_to_u8`] / [`ratchet::u8_to_grade`]
//!   reuse the same direction so the engine writes the same u8s the
//!   read endpoint expects.

pub mod cron;
pub mod ratchet;
pub mod recompute;
pub mod scoring;

pub use cron::{spawn_trust_engine_cron, tick, TrustEngineConfig};
pub use ratchet::{ratchet_grade, RatchetDecision};
pub use recompute::recompute_worker;
pub use scoring::{compute_score, ScoreBreakdown};
