//! `argo-mcp-image-gen` — thin MCP server exposing
//! `image.generate` + `image.edit` over stdio.
//!
//! Backends:
//! - [`OpenAiCompatibleBackend`] talks to DALL-E 3 / SD-class
//!   endpoints via the openai-compat REST shape
//!   (`/images/generations`, `/images/edits`).
//! - [`StubBackend`] returns deterministic placeholder bytes for
//!   tests.
//!
//! Each tool call writes a `cost_estimate` audit row BEFORE the
//! upstream API call (see [`server::APPROVAL_THRESHOLD_CENTS`]).
//! With `--require-confirm` set at boot, calls with an estimate
//! ≥ 20¢ short-circuit with an `ApprovalRequired` error.
//!
//! Verticals must declare `Capability::ImageGenerate` in their
//! manifest to dispatch these tools.
//!
//! # Config
//!
//! ```toml
//! [mcp.image_gen]
//! backend = "openai"  # "openai" | "stable_diffusion" | "stub"
//! api_base = "https://api.openai.com/v1"
//! model_default = "dall-e-3"
//! size_default = "1024x1024"
//! ```
//!
//! # Example
//!
//! ```
//! use argo_mcp_image_gen::{AuditSink, ImageGenMcpServer, StubBackend};
//! use std::sync::Arc;
//! let server = ImageGenMcpServer::new(
//!     Arc::new(StubBackend),
//!     "dall-e-3",
//!     "1024x1024",
//!     false,
//!     AuditSink::in_memory(),
//! );
//! assert_eq!(argo_mcp::McpServerService::list_tools(&server).len(), 2);
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod backend;
pub mod config;
pub mod server;

pub use backend::{
    estimate_cost_cents, ImageBackend, ImageResult, OpenAiCompatibleBackend, StubBackend,
};
pub use config::{BackendKind, ImageGenConfig};
pub use server::{AuditSink, ImageGenMcpServer, APPROVAL_THRESHOLD_CENTS};
