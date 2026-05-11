//! Signal MCP connector.
//!
//! Typed REST client for Signal messaging routed through the
//! [`signal-cli`](https://github.com/AsamK/signal-cli) local HTTP / JSON-RPC
//! daemon. Signal is the operator-requested high-privacy approval channel —
//! sister to iMessage (per-device) but cross-platform and end-to-end-encrypted
//! by default. Used by HIPAA-adjacent vertical pilots that need Signal as the
//! approval channel.
//!
//! ```no_run
//! use argo_mcp_connector_signal::SignalConnector;
//!
//! # async fn doc() -> argo_types::ArgoResult<()> {
//! let signal = SignalConnector::new(
//!     "http://127.0.0.1:8080".into(),
//!     "+15551234567".into(),
//! );
//! let receipt = signal.send_message("+15557654321", "approved").await?;
//! let _ts = receipt.timestamp;
//! let inbox = signal.receive().await?;
//! for env in inbox {
//!     if let Some(body) = env.body {
//!         println!("{}: {body}", env.source_number);
//!     }
//! }
//! # Ok(()) }
//! ```
//!
//! ## Wire-protocol shape
//!
//! The connector ONLY speaks to a loopback signal-cli daemon (default
//! `http://127.0.0.1:8080`) and never reaches Signal's infrastructure
//! directly. signal-cli exposes a JSON-RPC-over-HTTP API; each operation is a
//! POST or GET against `/{daemon_url}/api/v1/...`. Daemon install and the
//! one-time `signal-cli link` registration flow are the operator's job —
//! see the crate `README.md` for the runbook.
//!
//! ## Out of scope for v1
//!
//! - Bundled signal-cli install (operator runs it).
//! - Group create / admin (only list + send-to).
//! - Attachment upload (text only for v1).
//! - Registration / linking flow (operator does this once).
//! - Long-poll receive (caller polls every N seconds).
//! - Gateway dispatcher wiring (separate brief).

#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod client;
mod types;

pub use client::SignalConnector;
pub use types::{
    SignalAttachmentMeta, SignalGroup, SignalIncomingMessage, SignalMessageReceipt,
};
