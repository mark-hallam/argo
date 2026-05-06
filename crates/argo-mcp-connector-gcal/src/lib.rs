//! Google Calendar MCP connector.
mod client;
mod oauth;
mod service;

pub use client::{GcalClient, GcalConfig};
pub use oauth::{GoogleOAuthConfig, GCAL_OAUTH_SCOPES};
pub use service::GcalMcpService;
