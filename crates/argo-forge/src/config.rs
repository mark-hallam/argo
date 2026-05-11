//! Tunable constants for the Forge orchestration crate (Stream
//! `mission-forge-operator-feedback-eval-signal`, MISSION_BACKLOG
//! Section 3 Claim 4 #85).
//!
//! All constants here are intentionally documented as adjustable
//! knobs — they were chosen as v1 starting points, not load-bearing
//! invariants. If you change one, document the rationale in the PR
//! body so future operators can audit drift over time.

/// Loss-function weight applied to operator-feedback eval cases vs
/// synthesised cases. v1 = 1.5×: an operator who actually clicked
/// thumbs-up/down on a real action is a higher-signal label than a
/// synthesised example. Tunable; document the rationale if you bend
/// the dial.
pub const FEEDBACK_CASE_WEIGHT: f32 = 1.5;

/// Default look-back window (in days) for
/// `argo-forge::feedback_eval::ingest_operator_feedback`. v1 = 14
/// days: long enough to catch trends after a deployment, short
/// enough that stale "I hated this last quarter" votes don't drown
/// out recent signal. Tunable; the per-call `window: Duration`
/// parameter overrides this default for cron-style runs.
pub const FEEDBACK_WINDOW_DAYS: i64 = 14;
