//! Shared utilities for Argo MCP connectors.
//!
//! Currently exposes only the retry-with-backoff primitive used to absorb
//! transient transport-level failures (5xx, 429, connection errors) before
//! they surface to the agent. The connector keeps its public API unchanged;
//! the retry layer wraps the outbound call internally.
//!
//! See `retry::retry_with_policy` for the entry point and
//! `retry::default_http_policy` / `retry::is_http_retriable` for the HTTP
//! defaults a connector usually wants.

#![deny(missing_docs)]

pub mod retry;

#[cfg(test)]
mod retry_tests;

pub use retry::{
    default_http_policy, is_http_retriable, is_tonic_retriable, retry_with_policy, RetryPolicy,
};
