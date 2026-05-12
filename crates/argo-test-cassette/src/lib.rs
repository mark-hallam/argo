//! VCR-style cassette fixtures for Argo integration tests.
//!
//! Records real HTTP exchanges to JSON files on disk; replays them in CI
//! without touching the network. Secrets are redacted before write, so
//! cassettes are safe to commit. See the runbook at
//! `docs/skycot/runbook/cassette-fixtures.md` for the full migration recipe.
//!
//! # Modes
//!
//! Mode is read from `ARGO_TEST_CASSETTE_MODE`:
//! - `replay` (default): pop pre-recorded exchanges; no network.
//! - `record`: hit the real API, persist the redacted exchange.
//! - `passthrough`: hit the real API, do not persist (debug only).
//!
//! # Cassette JSON shape
//!
//! ```json
//! {
//!   "version": "1",
//!   "exchanges": [{
//!     "request": {
//!       "method": "POST", "url": "https://api.example.com/v1/x",
//!       "headers": { "authorization": "<REDACTED>" },
//!       "body_sha256": "…", "body_json_redacted": { "api_key": "<REDACTED>" }
//!     },
//!     "response": { "status": 200, "headers": {}, "body_json": { "ok": true } },
//!     "recorded_at": "2026-05-12T03:00:00Z"
//!   }]
//! }
//! ```
//!
//! # Example
//!
//! ```no_run
//! use argo_test_cassette::{Cassette, CassetteRecorder};
//! use std::collections::BTreeMap;
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let cassette = Cassette::load("fixtures/x.cassette.json")?;
//! let mut recorder = CassetteRecorder::new(cassette);
//! let response = recorder
//!     .execute("POST", "https://api.example.com/v1/x", BTreeMap::new(), Vec::new())
//!     .await?;
//! assert_eq!(response.status, 200);
//! # Ok(()) }
//! ```

pub mod cassette;
pub mod recorder;

pub use cassette::{
    redact_body_json, redact_headers, Cassette, CassetteError, CassetteExchange, CassetteRequest,
    CassetteResponse, Result, DEFAULT_HEADER_ALLOWLIST, REDACTED_PLACEHOLDER,
};
pub use recorder::{build_request_record, CassetteMode, CassetteRecorder};

#[cfg(test)]
#[path = "cassette_tests.rs"]
mod cassette_tests;
