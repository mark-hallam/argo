//! `argo-memory` — content-addressed memory store with locks (§14).
//!
//! Argo's memory model is four canonical files (`MEMORY.md`, `USER.md`,
//! `SOUL.md`, `AGENTS.md`) made of append-only [`MemoryEntry`] rows. New
//! entries supersede their predecessors via `superseded_by`; reverting is
//! a forward operation (write a new entry pointing at the prior version's
//! body).
//!
//! Locks (Amendment 3, §14.2) gate writes:
//!
//! - `Lock::None` — agent may freely edit.
//! - `Lock::UserProtected` — agent edits rejected with
//!   [`argo_types::ArgoError::UserProtected`]. Only an explicit
//!   [`PrincipalOverride::Force`] can write.
//! - `Lock::ContentLocked` — agent may edit, but the new entry MUST
//!   reference the previous entry's id via `supersede`.
//!
//! ```
//! # async fn doc() -> argo_types::ArgoResult<()> {
//! use std::sync::Arc;
//! use argo_memory::{MemoryFile, MemoryStore};
//! use argo_store::SessionStore;
//! let store = SessionStore::open(":memory:").await?;
//! store.migrate().await?;
//! let memory = MemoryStore::new(Arc::new(store));
//! assert_eq!(MemoryFile::Memory.filename(), "MEMORY.md");
//! let _ = memory;
//! # Ok(()) }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod context;
mod embedding;
mod entry;
mod layers;
mod row;
pub mod shared_team;
mod skill_candidate;
mod store;
mod summary;

#[cfg(test)]
mod layer_tests;
#[cfg(test)]
mod tests;

pub use context::ContextPack;
pub use embedding::{EmbeddingProvider, Voyage3EmbeddingProvider};
pub use entry::{MemoryEntry, MemoryFile, PrincipalOverride};
pub use layers::{LayeredMemoryEntry, MemoryLayer, ScoredMemoryEntry};
pub use shared_team::{shared_team, SharedEntry, SharedTeamNs};
pub use skill_candidate::{
    parse_skill_candidate, SkillCandidate, SCHEMA_KEY_LEARNED_SKILL_CANDIDATE,
};
pub use store::MemoryStore;
pub use summary::{
    HaikuSummaryProvider, MemorySummaryProvider, MonthlySummaryReport, YearMonth,
    NIGHTLY_SUMMARY_CRON,
};
