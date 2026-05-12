//! `argo-team-headline` — vertical-aware Today-block headline
//! generator for the Team page.
//!
//! Given a worker id + vertical id + open [`SessionStore`], returns
//! `2-3` bullets summarising the last 24 hours plus an optional
//! Tomorrow line. Bullets are computed from `audit_record` rows
//! (joined via `session.worker_id`) and the next-due duty in
//! `role_duty`. The KPI mapping per vertical is **hardcoded** in
//! `kpi_table::bullets_for_vertical` — v1 ships with one entry per
//! vertical listed in the E3-c brief. The architectural target is
//! to read KPI selection from each vertical's manifest; that
//! migration is tracked in `mission-enterprise-team-headline-manifest`.
//!
//! ```no_run
//! # async fn doc() -> argo_types::ArgoResult<()> {
//! use std::sync::Arc;
//! use argo_identity::WorkerId;
//! use argo_store::SessionStore;
//! use argo_team_headline::generate_team_headline;
//! let store = Arc::new(SessionStore::open(":memory:").await?);
//! store.migrate().await?;
//! let worker = WorkerId::new();
//! let headline = generate_team_headline(worker, "gtm-engineer", &store).await?;
//! assert!(headline.bullets.len() <= 3);
//! # Ok(()) }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

use std::sync::Arc;

use argo_identity::WorkerId;
use argo_store::SessionStore;
use argo_types::ArgoResult;
use chrono::Utc;
use serde::{Deserialize, Serialize};

mod compute;
mod kpi_table;
mod tomorrow;

#[cfg(test)]
mod lib_tests;

pub use compute::ActivityCounts;
pub use kpi_table::{bullets_for_vertical, BulletKind};

/// Maximum bullets returned. Brief: "2-3 bullets per worker".
pub const MAX_BULLETS: usize = 3;

/// Maximum bullet length in chars — keeps the team-card layout
/// from wrapping. Bullets longer than this are silently truncated
/// at a UTF-8 char boundary by [`crop_bullet`].
pub const MAX_BULLET_CHARS: usize = 60;

/// Today-block headline returned per worker on the Team page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamHeadline {
    /// 1-3 bullets summarising the worker's last 24 hours. Empty
    /// activity yields a single `"No activity yet."` bullet.
    pub bullets: Vec<String>,
    /// Optional "Tomorrow" line: the next-due duty in the next 24
    /// hours, formatted as `"<description> at HH:MM"`.
    pub tomorrow: Option<String>,
}

/// Build the Today-block headline for one worker.
///
/// Reads `audit_record` rows for the worker in the last 24 hours
/// and the next-due row in `role_duty`. Bullet selection is driven
/// by the v1 hardcoded KPI table in [`bullets_for_vertical`]; any
/// unknown vertical falls back to the generic 3-bullet shape.
pub async fn generate_team_headline(
    worker_id: WorkerId,
    vertical_id: &str,
    store: &Arc<SessionStore>,
) -> ArgoResult<TeamHeadline> {
    let now = Utc::now();
    let counts = compute::activity_counts_for_worker(store, worker_id, now).await?;
    let bullets = render_bullets(vertical_id, &counts);
    let tomorrow = tomorrow::next_duty_for_worker(store, worker_id, now).await?;
    Ok(TeamHeadline { bullets, tomorrow })
}

/// Pure shaping step: pick the vertical's KPI list, format each
/// bullet, drop `None`s, cap at [`MAX_BULLETS`], fall back to
/// `"No activity yet."` if nothing renders. Public-`pub(crate)`
/// for unit tests; not part of the external API surface.
pub(crate) fn render_bullets(vertical_id: &str, counts: &ActivityCounts) -> Vec<String> {
    let kinds = bullets_for_vertical(vertical_id);
    let mut out: Vec<String> = kinds
        .iter()
        .filter_map(|k| k.format(counts))
        .map(crop_bullet)
        .take(MAX_BULLETS)
        .collect();
    if out.is_empty() {
        out.push("No activity yet.".to_string());
    }
    out
}

/// Truncate a bullet at the [`MAX_BULLET_CHARS`] char boundary
/// (UTF-8 safe). Bullets emitted by formatters are designed to fit
/// already; this is the belt-and-braces guardrail.
fn crop_bullet(s: String) -> String {
    if s.chars().count() <= MAX_BULLET_CHARS {
        return s;
    }
    s.chars().take(MAX_BULLET_CHARS).collect()
}
