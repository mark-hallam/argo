//! Replicate MCP connector.
//!
//! Typed REST client for [Replicate](https://replicate.com)'s prediction
//! API. Replicate is the universal Hugging-Face-model-hosting endpoint —
//! any model published there (image, audio, video, LLM, vision) can be
//! called through one API shape, so a single connector unlocks the entire
//! long-tail of HF models for SKYCOT verticals.
//!
//! ```no_run
//! use argo_mcp_connector_replicate::ReplicateConnector;
//! use std::time::Duration;
//!
//! # async fn doc() -> argo_types::ArgoResult<()> {
//! let rc = ReplicateConnector::new("r8_xxx".into())?;
//! let prediction = rc
//!     .run_prediction(
//!         "meta/meta-llama-3-8b-instruct",
//!         serde_json::json!({"prompt": "hello", "max_new_tokens": 1}),
//!         Duration::from_secs(60),
//!     )
//!     .await?;
//! println!("status = {:?}, output = {:?}", prediction.status, prediction.output);
//! # Ok(()) }
//! ```
//!
//! ## Auth
//!
//! Replicate uses `Authorization: Token <api_token>` (NOT `Bearer`). Their
//! docs explicitly use the `Token` scheme; getting this wrong yields 401
//! on every call.
//!
//! ## Polling model
//!
//! [`ReplicateConnector::run_prediction`] polls `GET /v1/predictions/<id>`
//! with exponential backoff `[1, 2, 4, 8, 15]s` (15s ceiling) until the
//! prediction reaches a terminal state (`Succeeded` / `Failed` /
//! `Canceled`) or the caller-supplied timeout elapses. Callers that need
//! to pace polling themselves can use [`ReplicateConnector::start_prediction`]
//! + [`ReplicateConnector::get_prediction`] directly.
//!
//! ## Output shape
//!
//! [`Prediction::output`] is intentionally [`serde_json::Value`] — model
//! outputs vary wildly (image URLs / text / audio URLs / JSON arrays /
//! token streams) and the calling vertical knows the shape per-model.
//! The connector does NOT normalize.
//!
//! ## Cancel semantics
//!
//! [`ReplicateConnector::cancel_prediction`] returning `Ok(())` means
//! "cancellation accepted", not "cancellation completed". The prediction
//! may emit a `Canceled`-status row after some delay; poll
//! [`ReplicateConnector::get_prediction`] to confirm the terminal state.
//!
//! ## Pricing visibility
//!
//! [`PredictionMetrics::predict_time`] is the billing-relevant duration
//! (in seconds). Surface it so the calling vertical can attribute spend
//! per call.
//!
//! ## License surfacing
//!
//! [`ModelInfo::license_url`] lets callers refuse models with restrictive
//! licenses. The connector itself does NOT enforce — the vertical decides
//! based on its own policy.
//!
//! ## Account cache
//!
//! [`ReplicateConnector::account`] memoizes successful responses for 60s
//! (configurable in tests) to avoid auth-churn on parallel calls.
//!
//! ## Errors
//!
//! - `404` on get / cancel → [`argo_types::ArgoError::Provider`] with a
//!   `not-found` marker (the brief calls for a typed not-found surfacing
//!   without a panic).
//! - `401` on any call → [`argo_types::ArgoError::Provider`] prefixed
//!   with `"replicate auth"` so triage can distinguish bad-token from
//!   generic provider failure.
//! - Polling-loop timeout → [`argo_types::ArgoError::Provider`] with a
//!   `timeout` marker.

#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod client;
mod poll;
mod types;

pub use client::ReplicateConnector;
pub use poll::POLL_BACKOFF_MS;
pub use types::{
    AccountInfo, ModelInfo, Prediction, PredictionId, PredictionMetrics, PredictionStatus,
};
