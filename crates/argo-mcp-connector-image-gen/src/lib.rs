//! Image-generation MCP connector. One tool, three backends.
//!
//! Exposes `image-gen.generate_image(prompt, style?, size?, backend?)`.
//! Three thin HTTP clients route to OpenAI DALL-E 3 (default),
//! Stability AI, or Replicate. The service downloads PNG bytes and
//! writes them to `<workspace_root>/.argo/generated/<sha256>.png`,
//! returning a `file://` URL.
//!
//! # Backend selection
//!
//! 1. The `backend` argument wins if it names a known backend.
//! 2. Otherwise `IMAGE_GEN_BACKEND` env (`openai` | `stability` |
//!    `replicate`) wins.
//! 3. Otherwise the default `openai` is used when `OPENAI_API_KEY` is
//!    set, then the first backend with credentials, else a
//!    schema-only error.
//!
//! # Required environment per backend
//!
//! | Backend     | Required env                                |
//! |-------------|---------------------------------------------|
//! | `openai`    | `OPENAI_API_KEY`                            |
//! | `stability` | `STABILITY_API_KEY`                         |
//! | `replicate` | `REPLICATE_API_TOKEN`                       |
//!
//! # Capability
//!
//! Dispatch path uses
//! `Capability::ToolDispatch{tool_name_glob: "image-gen.*"}`. Each
//! backend talks to a different host, so the operator should grant
//! `Capability::NetEgress` for `api.openai.com`, `api.stability.ai`,
//! and `api.replicate.com` as needed.
//!
//! ```no_run
//! # async fn doc() -> argo_mcp::McpResult<()> {
//! use argo_mcp::{CallToolParams, McpServerService};
//! use argo_mcp_connector_image_gen::ImageGenService;
//!
//! let svc = ImageGenService;
//! assert_eq!(svc.list_tools().len(), 1);
//! let _ = svc.call_tool(CallToolParams {
//!     name: "image-gen.generate_image".into(),
//!     arguments: serde_json::json!({"prompt": "an argonaut at sunrise"}),
//! }).await?;
//! # Ok(()) }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod auth;
mod b64;
pub mod client;
mod schemas;
mod service;

pub use auth::{Backend, BackendConfig};
pub use client::ImageGenClient;
pub use service::ImageGenService;
