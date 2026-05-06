//! Input + lifecycle helpers for the `D` dashboard overlay
//! (Stream `claude-tui-realtime-dashboard`). Sibling of
//! `input_h_stack.rs` / `input_replay.rs`.
//!
//! Three responsibilities:
//! 1. Close + nav: [`try_handle_dashboard_overlay`] is called from
//!    the input event loop and returns `true` when it consumed a
//!    key.
//! 2. Open: [`open_dashboard_overlay`] flips the `dashboard_overlay`
//!    field on `ViewerState` to `Some(...)` so subsequent draws
//!    render the overlay.
//! 3. Refresh: [`refresh_dashboard`] is called from the existing
//!    1 s tick in `input.rs::run`. It fetches the 3 panes' data
//!    in parallel:
//!      - workers + pending → HTTP via [`crate::overlays_h_stack::fetch_panel`]
//!        (routes `/orchestrator/workers` + `/orchestrator/approvals/pending`
//!        ship in a follow-up; until then panes render
//!        `Unreachable` cleanly)
//!      - audit tail → direct in-process query via
//!        [`argo_audit::tail_rows_filtered`].
//!
//! The 1 s tick is the existing event-loop refresh hook (not a
//! new background task). A v1.x WebSocket transport on top of
//! [`argo_audit::AuditSink::subscribe`] would let the audit pane
//! stream at sub-second granularity without changing this surface.
use crate::app::ViewerState;
use crate::overlays_dashboard::{
    payload_headline, AuditTailEntry, DashboardState, AUDIT_TAIL_CAPACITY,
    AUDIT_TAIL_DEFAULT_VISIBLE,
};
use crate::overlays_h_stack::{fetch_panel, gateway_url};
use argo_store::SessionStore;
use crossterm::event::KeyCode;

/// Handle one keypress when the dashboard overlay is open. Returns
/// `true` if the key was consumed (so the input loop can `continue`
/// without falling through to the global keymap).
///
/// Close keys: `Esc` / `D` / `q`.
/// Scroll keys: `j`/`k`/Down/Up/PageDown/PageUp/g/G/Home/End — all
/// scroll the audit-tail pane (the one pane the operator interacts
/// with; workers + pending are read-only summaries).
pub(crate) fn try_handle_dashboard_overlay(state: &mut ViewerState, code: KeyCode) -> bool {
    let Some(dash) = state.dashboard_overlay.as_mut() else {
        return false;
    };
    match code {
        KeyCode::Esc | KeyCode::Char('D') | KeyCode::Char('q') => {
            state.dashboard_overlay = None;
        }
        KeyCode::Char('j') | KeyCode::Down => dash.scroll_audit_down(),
        KeyCode::Char('k') | KeyCode::Up => dash.scroll_audit_up(),
        KeyCode::PageDown => {
            for _ in 0..AUDIT_TAIL_DEFAULT_VISIBLE {
                dash.scroll_audit_down();
            }
        }
        KeyCode::PageUp => {
            for _ in 0..AUDIT_TAIL_DEFAULT_VISIBLE {
                dash.scroll_audit_up();
            }
        }
        KeyCode::Char('g') | KeyCode::Home => {
            dash.audit_scroll = 0;
        }
        KeyCode::Char('G') | KeyCode::End => {
            dash.audit_scroll = dash.audit_tail.len().saturating_sub(1);
        }
        // Consume any other key — the dashboard is the operator's
        // "always-on" tab; we don't want a stray keypress to
        // trigger the global keymap (which could open another
        // overlay on top of this one and surprise the operator).
        _ => {}
    }
    true
}

/// Initialize the dashboard overlay state. Called from the bottom
/// `D` match arm in `input.rs::run`. Subsequent refresh ticks
/// populate the panes; the first frame renders Loading hints.
pub(crate) fn open_dashboard_overlay(state: &mut ViewerState) {
    state.dashboard_overlay = Some(DashboardState::new());
}

/// One refresh tick: fetch fresh data into the dashboard panes if
/// the overlay is open. No-op when closed.
///
/// - `workers` panel hits `/orchestrator/workers` (route ships in
///   a v1.x follow-up; until then the call returns `Unreachable`
///   and the pane renders cleanly).
/// - `pending` panel hits `/orchestrator/approvals/pending`
///   (likewise queued).
/// - `audit_tail` is populated in-process from
///   [`argo_audit::tail_rows_filtered`].
pub(crate) async fn refresh_dashboard(state: &mut ViewerState, store: &SessionStore) {
    if state.dashboard_overlay.is_none() {
        return;
    }
    let base = gateway_url();
    let workers = fetch_panel(&base, "/orchestrator/workers", true).await;
    let pending =
        fetch_panel(&base, "/orchestrator/approvals/pending", true).await;
    let audit = fetch_audit_tail(store).await;
    if let Some(dash) = state.dashboard_overlay.as_mut() {
        dash.workers = workers;
        dash.pending = pending;
        dash.set_audit_tail(audit);
    }
}

/// Fetch up to [`AUDIT_TAIL_CAPACITY`] most-recent audit rows from
/// the local SQLite store and project them into
/// [`AuditTailEntry`]. Errors → empty Vec (the pane renders
/// "(no audit rows yet)" cleanly).
pub(crate) async fn fetch_audit_tail(store: &SessionStore) -> Vec<AuditTailEntry> {
    let now_micros = chrono::Utc::now().timestamp_micros();
    let filters = argo_audit::TailFilters::default();
    let rows = match argo_audit::tail_rows_filtered(store, AUDIT_TAIL_CAPACITY as u32, &filters)
        .await
    {
        Ok(rows) => rows,
        Err(_) => return Vec::new(),
    };
    rows.into_iter()
        .map(|r| AuditTailEntry {
            kind: r.kind.clone(),
            age: crate::overlays_dashboard::human_age(now_micros, r.created_at_micros),
            headline: payload_headline(&r.kind, &r.payload),
            session_short: r.session_id.as_ref().map(|s| s.chars().take(8).collect()),
        })
        .collect()
}

#[cfg(test)]
#[path = "input_dashboard_tests.rs"]
mod tests;
