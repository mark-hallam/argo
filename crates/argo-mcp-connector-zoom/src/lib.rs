//! Zoom MCP connector.
//!
//! Typed REST client for the Zoom v2 `meetings` / `recordings` / `users`
//! surface. Authenticates via Server-to-Server OAuth (account-credentials
//! grant), not user-context OAuth — there is no consent screen and no
//! per-user refresh-token rotation. The Server-to-Server app must be
//! created in the Zoom marketplace under the operator's account; the
//! account-id, client-id, and client-secret then live in operator
//! credentials.
//!
//! ```no_run
//! use argo_mcp_connector_zoom::{ZoomConnector, ZoomMeetingKind};
//!
//! # async fn doc() -> argo_types::ArgoResult<()> {
//! let zoom = ZoomConnector::new(
//!     "account-id".into(),
//!     "client-id".into(),
//!     "client-secret".into(),
//! )?;
//! let _meetings = zoom
//!     .list_meetings("me", ZoomMeetingKind::Upcoming)
//!     .await?;
//! # Ok(()) }
//! ```
//!
//! ## Auth
//!
//! Server-to-Server OAuth issues short-lived (~1h) access tokens against
//! `https://zoom.us/oauth/token?grant_type=account_credentials&account_id=<id>`
//! with HTTP Basic auth (client_id:client_secret). The connector caches the
//! token in-memory, reuses it until `expires_in - 60s` elapsed, and
//! transparently refreshes on a 401 before propagating the error.
//!
//! ## Time
//!
//! `ZoomMeetingDraft::start_time` is sent as ISO-8601 with a `Z` suffix.
//! The `timezone` field is a separate IANA tz string (e.g.
//! `"America/Los_Angeles"`) that Zoom uses for the user-visible time —
//! it does NOT shift `start_time`.
//!
//! ## Recording window
//!
//! Zoom hard-caps the recording-list window at 30 days; the connector
//! returns a typed error for wider windows rather than silently truncating.

#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod auth;
mod client;
pub mod types;

pub use auth::ZoomS2sAuth;
pub use client::ZoomConnector;
pub use types::{
    ZoomMeeting, ZoomMeetingDraft, ZoomMeetingKind, ZoomRecording, ZoomRecordingFile, ZoomUser,
    ZoomUserStatus,
};
