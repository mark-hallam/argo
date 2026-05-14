//! `argo-plugin` — plugin protocol + subprocess host (§11).
//!
//! Argo runs plugins as **subprocesses** that speak **line-delimited
//! JSON-RPC 2.0** over stdin/stdout (Invariant 1: polyglot core). A
//! plugin in any language implements:
//!
//! 1. Read a JSON object per line from stdin.
//! 2. If the object's `method` is `handshake`, reply with the protocol
//!    version + plugin name on stdout.
//! 3. For each subsequent method, run the request and write a
//!    JSON-RPC response.
//! 4. On `shutdown`, exit cleanly.
//!
//! Phase-1 surface here ships [`PluginHost`] (Argo → plugin direction
//! with handshake, typed methods, crash detection, per-call timeout).
//! Plugin → Argo callbacks (`enqueue_steer`, `dispatch_tool`,
//! `telemetry_event`, …) wire in with the next §11 PR alongside the
//! Python + TypeScript SDKs.
//!
//! ```no_run
//! # async fn doc() -> argo_plugin::PluginResult<()> {
//! use argo_plugin::PluginHost;
//! let host = PluginHost::start("/path/to/my-plugin", &[]).await?;
//! let v = host.call("ping", serde_json::json!({})).await?;
//! assert_eq!(v["pong"], true);
//! host.shutdown().await?;
//! # Ok(()) }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod error;
mod host;
mod protocol;
pub mod registry;

pub use error::{PluginError, PluginResult};
pub use host::{PluginHost, DEFAULT_CALL_TIMEOUT};
pub use protocol::{
    HandshakeRequest, HandshakeResponse, JsonRpcError, JsonRpcRequest, JsonRpcResponse,
    PreToolCallVerdict, PROTOCOL_VERSION,
};
pub use registry::{PluginManifestEntry, PluginRegistry, PluginRegistryConfig};

#[cfg(test)]
mod registry_tests;
