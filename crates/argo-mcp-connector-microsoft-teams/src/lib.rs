//! Microsoft Teams MCP connector.
//!
//! Typed REST client for the Microsoft Graph v1.0 surface covering Teams
//! channels, channel messages, search, members, and Online Meetings.
//! Authenticates with an Azure AD bearer access token supplied by the
//! caller — the connector does not acquire or refresh tokens itself.
//!
//! ```no_run
//! use argo_mcp_connector_microsoft_teams::TeamsConnector;
//!
//! # async fn doc() -> argo_types::ArgoResult<()> {
//! let teams = TeamsConnector::new("aad-access-token".into());
//! let _teams = teams.list_teams().await?;
//! # Ok(()) }
//! ```
//!
//! ## Auth scopes (Azure AD)
//!
//! - `Team.ReadBasic.All` — read joined teams.
//! - `Channel.ReadBasic.All` — list channels in a team.
//! - `ChannelMessage.Send` / `ChatMessage.Send` — post messages.
//! - `Chat.Read` + `Mail.Read` — unified `/search/query` hits across DMs
//!   + Outlook (Graph search returns both under `chatMessage`).
//! - `OnlineMeetings.ReadWrite` — create Teams Online Meetings.
//! - `Calendars.ReadWrite` — only if the caller also writes a calendar
//!   event alongside the meeting; this connector does NOT touch the
//!   Calendar API surface.
//!
//! ## Message body: text vs HTML
//!
//! Graph's `chatMessage.body.contentType` accepts `text` or `html`.
//! [`TeamsConnector::post_message`] sends plain text;
//! [`TeamsConnector::post_message_html`] sends inline HTML and is the
//! right call for messages that need bold, links, or `@`-mentions
//! rendered. The connector does not sanitise or escape HTML — the caller
//! must produce trusted markup.
//!
//! ## Online Meeting ≠ calendar event
//!
//! [`TeamsConnector::schedule_meeting`] creates a Teams Online Meeting
//! object: it has a `join_web_url` and exists independently of any
//! calendar. To put the meeting on a calendar, the caller hits the Graph
//! Calendar API (`/me/events`) with the meeting's join URL embedded —
//! that surface is intentionally out of scope here.
//!
//! ## Safety posture
//!
//! Channel-creation is NOT exposed. Workers post to channels that already
//! exist; spawning channels is a moderator concern, not a worker one.

#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod client;
pub mod types;

pub use client::{TeamsConnector, GRAPH_API_BASE_URL};
pub use types::{
    format_graph_time, Channel, Message, MessageContentType, OnlineMeeting, Team, TeamMember,
};
