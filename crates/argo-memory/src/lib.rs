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
#[cfg(feature = "dreams-backend")]
pub mod dreams_backend;
mod embedding;
mod entry;
mod layers;
mod row;
pub mod shared_team;
mod skill_candidate;
mod store;
mod store_vertical;
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

/// Append a memory entry, then classify it via
/// [`argo_store::classification`] in one shot (Stream
/// `claude-data-classification`). When `class` is `None` this is
/// just a regular [`MemoryStore::append`] — the tag insert is
/// skipped. Errors during the classification insert are returned
/// without rolling back the append (the entry exists, the tag
/// didn't); the caller may retry the tag via
/// `argo classify <kind>:<id> <class>` if it matters.
///
/// Takes `Arc<SessionStore>` so this function can reach the raw
/// sqlite pool for the classification insert without widening
/// `MemoryStore`'s public API. Internally it constructs a
/// short-lived `MemoryStore` for the append.
///
/// `tagged_by` is optional — pass the worker / operator id when
/// known, `None` for system-emitted writes.
pub async fn append_with_class(
    store: std::sync::Arc<argo_store::SessionStore>,
    workspace_id: argo_types::WorkspaceId,
    file: MemoryFile,
    body: String,
    learned_in: Option<argo_types::SessionId>,
    class: Option<argo_identity::DataClass>,
    tagged_by: Option<&str>,
) -> argo_types::ArgoResult<MemoryEntry> {
    let pool_handle = store.pool().clone();
    let memory = MemoryStore::new(store);
    let entry = memory.append(workspace_id, file, body, learned_in).await?;
    if let Some(c) = class {
        argo_store::classification::insert_tag(
            &pool_handle,
            "memory_entry",
            &entry.id.to_string(),
            c.as_str(),
            tagged_by,
        )
        .await?;
    }
    Ok(entry)
}
