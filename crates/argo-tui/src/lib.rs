//! `argo-tui` — interactive terminal UI (§20). First cut: a
//! read-only session viewer that lists recent sessions on the
//! left and renders the highlighted session's header on the right.
//!
//! The TUI is intentionally tight — ratatui + crossterm, no
//! threads beyond what tokio gives us. The full §20 surface
//! (sticky composer, live streaming, OSC-52 clipboard) lands in
//! follow-up PRs that bolt onto this skeleton.
//!
//! Entry: [`run_session_viewer`] takes an open `SessionStore` and
//! drives the event loop until the user quits with `q` / Ctrl-C.
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod actions;
mod app;
mod app_navigation;
mod clipboard;
mod input;
mod input_dashboard;
mod input_h_stack;
mod input_helpers;
mod input_overlay_dispatch;
mod input_replay;
mod live_event;
pub mod notifications;
mod overlay_text;
mod overlays;
mod overlays_budget;
mod overlays_dashboard;
mod overlays_h_stack;
mod overlays_handoffs;
mod overlays_replay;
mod overlays_sessions;
mod overlays_tables;
mod overlays_tool_errors;
mod render;
mod render_messages;
pub mod session_detail;
mod text_input;
pub mod theme;
mod viewer;

pub use app::ViewerState;
pub use theme::{from_config as theme_from_config, Theme};
pub use viewer::run_session_viewer;

/// Aux-overlay close-key dispatcher consolidating h_stack +
/// replay + dashboard. Lives here (rather than in `input.rs`) so
/// the event-loop file stays under the 500-LOC ceiling — same
/// trick #853 used to combine h_stack + replay close handling.
/// Returns `true` when the key was consumed by one of the aux
/// overlays so the caller can `continue`.
pub(crate) fn try_close_aux_overlay(
    state: &mut crate::app::ViewerState,
    code: crossterm::event::KeyCode,
) -> bool {
    crate::input_h_stack::try_close_h_stack_overlay(state, code)
        || crate::input_replay::try_handle_replay_overlay(state, code)
        || crate::input_dashboard::try_handle_dashboard_overlay(state, code)
}
