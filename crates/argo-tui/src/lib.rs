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
mod clipboard;
mod input;
mod live_event;
mod overlay_text;
mod overlays;
mod render;
mod text_input;
mod viewer;

pub use app::ViewerState;
pub use viewer::run_session_viewer;
