//! `argo-provider` — provider manifests + transports.
//!
//! Quirks live in TOML, not Rust (Invariant 4 — see CLAUDE.md). This crate
//! implements:
//!
//! - the manifest schema (§8.1) with TOML deserialization + cross-field
//!   validation,
//! - manifest-driven [`Transport`] implementations for all four v1 families
//!   (`openai-compatible`, `openai-responses`, `anthropic`, `bedrock`),
//! - six tool-call extractors covering OpenAI Chat Completions, OpenAI
//!   Responses, Anthropic, Bedrock, Hermes-style XML, and an LLM-extracted
//!   fallback.
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

mod anthropic;
mod bedrock;
/// Per-provider breaker state-transition observability + the
/// `ObservedBreakerTransport` composition wrapper.
pub mod breaker_observability;
/// Per-provider circuit breaker (Anthropic / OpenAI / Bedrock / …).
pub mod circuit;
mod discovery;
mod extractors;
mod heartbeat;
mod manifest;
mod openai_compat;
mod openai_responses;
/// reqwest connection-pool tuning knobs (env-driven).
pub mod pool_config;
mod sse;
/// Provider-transport trait, wire types, and `build_http_client` helper.
pub mod transport;

pub use anthropic::AnthropicTransport;
pub use bedrock::BedrockTransport;
pub use breaker_observability::{ObservedBreakerTransport, StateObserver};
pub use circuit::{Breaker, BreakerConfig, CircuitState, Permit};
pub use discovery::resolve_latest;
pub use extractors::{
    anthropic_tool_use, bedrock_tool_use, extract_tool_calls, llm_extracted, openai_response_tools,
    openai_tool_calls, xml_tool_call,
};
pub use heartbeat::HeartbeatMonitor;
pub use manifest::{
    AuthSpec, CapabilitiesSpec, DiscoverySpec, HooksSpec, ModelSpec, PreferRule, ProviderFamily,
    ProviderHeader, ProviderManifest, ReasoningSpec, StreamingMode, ToolCallFormat, ToolCallsSpec,
    TransportSpec,
};
pub use openai_compat::{event_to_deltas, OpenAiCompatTransport};
pub use openai_responses::OpenAiResponsesTransport;
pub use sse::SseBuffer;
pub use transport::{
    ProviderRequest, ProviderResponse, ProviderResponseDelta, Sampling, StopReason, ToolSchema,
    Transport,
};
