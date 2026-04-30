//! `argo-acp` — Agent Client Protocol server (§28.Y).
//!
//! Phase 4.5 ship: the library used by `argo acp serve` (§19 CLI).
//! Pieces:
//!
//! - **Wire envelopes** — JSON-RPC 2.0 [`AcpRequest`] /
//!   [`AcpResponse`] / [`AcpNotification`] / [`AcpError`].
//! - **Auth** ([`AuthScheme`], [`verify`]) — Copilot OAuth, Cursor
//!   token, generic bearer. Resolves to an [`argo_types::Principal::Acp`].
//! - **Multi-harness** ([`Harness`], [`HarnessCapabilities`]) —
//!   Copilot CLI, Cursor CLI, VS Code, Zed, JetBrains, generic. No
//!   single-harness hardcoding (§28.Y point 2).
//! - **Permission mapping** ([`AcpPermission`], [`map_permission`])
//!   — translates ACP permission requests onto Argo
//!   [`argo_types::Capability`] values for `argo_capability::CapabilityEngine`
//!   (§28.Y point 5).
//! - **Server dispatch** ([`AcpServer`], [`AcpService`]) — routes
//!   the four advertised methods (`session/send_message`,
//!   `session/list`, `session/fork`, plus streaming notifications).
//!
//! ```
//! # use argo_acp::{Harness, HarnessCapabilities, AuthScheme};
//! let caps = HarnessCapabilities::default_for(Harness::VsCode);
//! assert!(caps.send_message);
//! assert_eq!(Harness::CursorCli.id(), "cursor_cli");
//! let _ = AuthScheme::CopilotOAuth;
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod auth;
mod harness;
mod permission;
mod server;
mod wire;

#[cfg(test)]
mod tests;

pub use auth::{verify, AuthError, AuthScheme, AuthToken};
pub use harness::{Harness, HarnessCapabilities};
pub use permission::{map_permission, AcpPermission};
pub use server::{
    AcpServer, AcpService, ForkRequest, ListSessionsResponse, SendMessageRequest, StreamChunk,
};
pub use wire::{AcpError, AcpNotification, AcpRequest, AcpResponse, JSONRPC_VERSION};
