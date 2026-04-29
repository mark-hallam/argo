//! `argo-provider` — provider manifests + transports.
//!
//! Quirks live in TOML, not Rust (Invariant 4 — see CLAUDE.md). This crate
//! implements:
//!
//! - the manifest schema (§8.1) with TOML deserialization + cross-field
//!   validation,
//! - a manifest-driven [`Transport`] for the `openai-compatible` family
//!   (Phase 2 ships this one only — Anthropic / Bedrock / Responses are
//!   Agent F),
//! - six tool-call extractors — `openai_tool_calls` is the only one wired
//!   end-to-end this PR; the other five return
//!   `ArgoError::Provider("not yet implemented in Phase 2")`.
//!
//! ```
//! # fn doc() -> argo_types::ArgoResult<()> {
//! use argo_provider::{OpenAiCompatTransport, ProviderManifest};
//! let toml = include_str!("../../../manifests/providers/openai-compatible.toml");
//! let manifest = ProviderManifest::from_toml_str(toml)?;
//! let _t = OpenAiCompatTransport::from_manifest(manifest, "sk-test")?;
//! # Ok(()) }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod extractors;
mod manifest;
mod openai_compat;
mod transport;

pub use extractors::{
    anthropic_tool_use, bedrock_tool_use, extract_tool_calls, llm_extracted, openai_response_tools,
    openai_tool_calls, xml_tool_call,
};
pub use manifest::{
    AuthSpec, CapabilitiesSpec, DiscoverySpec, HooksSpec, ModelSpec, PreferRule, ProviderFamily,
    ProviderHeader, ProviderManifest, ReasoningSpec, StreamingMode, ToolCallFormat, ToolCallsSpec,
    TransportSpec,
};
pub use openai_compat::{event_to_deltas, OpenAiCompatTransport};
pub use transport::{
    ProviderRequest, ProviderResponse, ProviderResponseDelta, Sampling, StopReason, ToolSchema,
    Transport,
};
