//! Worker settings + danger-zone endpoints (Stream J11).
//!
//! Nine HTTP routes anchored under `/api/v1/workers/<id>/...` powering
//! E4-f's Settings tab and E3-b's pause/resume context menu:
//!
//! - `GET    /settings`     — bundled settings + advanced (operator-only).
//! - `PATCH  /channels`     — update notify channels + primary.
//! - `PATCH  /quiet-hours`  — update DND window.
//! - `PATCH  /budget`       — update daily / monthly USD caps.
//! - `PATCH  /autonomy`     — set conservative / balanced / aggressive.
//! - `POST   /pause`        — pause the worker.
//! - `POST   /resume`       — resume from pause or recover from archive.
//! - `POST   /archive`      — soft-delete (30-day recovery window).
//! - `DELETE /`             — hard-delete (operator + name-confirm header).
//!
//! Split for the 500-LOC ceiling per the brief's notes:
//!
//! - `routes_worker_settings_read`   — GET + the four PATCH handlers.
//! - `routes_worker_settings_danger` — pause / resume / archive / DELETE.
//! - `routes_worker_settings_helpers` — shared lookups + audit + envelope.
//!
//! ## v1 posture
//!
//! Per the brief's v1 column, settings mutations flow through the existing
//! audit infrastructure rather than a dedicated `worker_settings` table.
//! PATCH/POST handlers persist a `worker.<facet>.changed` audit row;
//! `GET /settings` reads back the latest row per facet and falls back to
//! built-in defaults when none exists. `worker.channels` continues to live
//! on the `Worker` struct (the only setting with a real column today).
//!
//! ## Sensible deviations from the brief
//!
//! - "Status → paused" persists as an audit-row marker (`worker.paused`)
//!   in v1; the live `worker.circuit_state` mutator lives in
//!   `argo-identity`, which is read-only for this brief. The Settings tab
//!   reads back paused/idle from the latest paused/resumed audit row.
//! - DELETE bypasses the heavyweight `worker_archive::archive_worker +
//!   force_delete_worker` two-step. The brief asks for a single
//!   hard-delete that removes memory + skills assignments and leaves
//!   audit; the gateway issues those DELETEs inline against the live
//!   `worker` row.
//! - "PATCH /autonomy conservative resets trust-ratchet" — the
//!   ratchet counter table doesn't exist yet; v1 emits a
//!   `worker.trust_ratchet.reset` audit row so the future consumer brief
//!   can pick it up. Flagged as `mission-enterprise-trust-ratchet-reset`.

#[path = "routes_worker_settings_helpers.rs"]
mod helpers;
#[path = "routes_worker_settings_read.rs"]
mod read;
#[path = "routes_worker_settings_danger.rs"]
mod danger;

pub(super) use danger::{delete_worker_endpoint, post_archive, post_pause, post_resume};
pub(super) use read::{
    get_settings, patch_autonomy, patch_budget, patch_channels, patch_quiet_hours,
};

#[cfg(test)]
#[path = "routes_worker_settings_tests.rs"]
mod tests;
