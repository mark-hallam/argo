//! `argo-audit` — hash-chained audit log + 17 event kinds (§17).
//!
//! Phase-1 audit lived inline in `argo-core`. This crate is the
//! canonical home: typed [`AuditKind`] enum (6 Phase-1 kinds + 11
//! v0.2 kinds per §17.1), [`AuditSink`] writer, and a chain
//! [`verify_session`] / [`verify_all`] that the `argo audit verify`
//! CLI calls.
//!
//! Chain shape:
//! `self_hash = sha256(parent_hash || kind || 0x00 || canonical_payload)`
//! per session. The first row in a session has
//! `parent_hash = 0…0`.
//!
//! ```
//! # async fn doc() -> argo_types::ArgoResult<()> {
//! use std::sync::Arc;
//! use argo_audit::{AuditKind, AuditSink};
//! use argo_store::SessionStore;
//! let store = Arc::new(SessionStore::open(":memory:").await?);
//! store.migrate().await?;
//! let sink = AuditSink::new(store);
//! sink.write(None, AuditKind::TurnStart, serde_json::json!({"k": 1}), None).await?;
//! # Ok(()) }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

/// Periodic evidence archival (#228, Tier S) — tar.gz bundle +
/// signed manifest + verify + optional prune.
pub mod archive;
mod event;
mod export;
/// Denormalized filter-dimension extraction shared by the write
/// path (`sink::AuditSink::write`) and the verify-chain denorm
/// sanity belt (`verify::verify_session`). Also re-used by the
/// J14 audit-feed read path's SQL-branch filter wiring.
pub mod filter_dims;
mod gap;
mod replay;
mod sink;
mod verify;
mod watchdog;

#[cfg(test)]
mod filter_dims_tests;
#[cfg(test)]
mod tests;

pub use event::AuditKind;
pub use export::{
    export_ndjson, rows_for_session, tail_rows, tail_rows_filtered, ExportedRow, TailFilters,
};
pub use gap::{detect_policy_denials, CapabilityGap, GapKind};
pub use replay::{filter_rows as filter_replay_rows, render as render_replay, ReplayFormat};
pub use sink::{compute_chain_hash, AuditEvent, AuditSink, AUDIT_BROADCAST_CAPACITY};
pub use verify::{verify_all, verify_session, ChainBreak, VerifyReport};
pub use watchdog::{spawn_watchdog, WatchdogConfig};
