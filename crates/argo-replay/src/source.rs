//! Session source — abstracts where replay reads turns from.
//!
//! Production impls hand-build a [`SessionSource`] over `argo-store`
//! to read from the SQLite session DB. Tests use [`InMemorySource`]
//! to feed pre-built turns directly.
use argo_types::{ArgoError, ArgoResult, SessionId};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// One turn from a recorded session — the inputs (user) + the
/// emitted assistant output + tool activity.
///
/// Replay re-invokes the assistant against the same `user_text`
/// (with optional model/tools/workspace overrides) and compares the
/// new `assistant_text` to the original.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnRecord {
    /// 0-indexed turn number within the session.
    pub turn: u32,
    /// User input for this turn (concatenated text content).
    pub user_text: String,
    /// Assistant output (concatenated text content).
    pub assistant_text: String,
    /// Tool calls emitted during the turn, in invocation order.
    pub tool_calls: Vec<ToolCallRecord>,
}

/// One tool invocation within a turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCallRecord {
    /// Tool name (e.g. `"file_read"`).
    pub name: String,
    /// JSON-encoded arguments.
    pub arguments_json: String,
    /// JSON-encoded result; `None` if the call errored out before
    /// returning a body.
    pub result_json: Option<String>,
    /// `true` if the tool returned an error.
    pub is_error: bool,
}

/// Source for `(session_id) → turns` lookups.
///
/// Decouples replay from `argo-store`: the production impl builds
/// the `Vec<TurnRecord>` from `message` / `tool_call` rows; tests
/// feed the records directly via [`InMemorySource`].
#[async_trait]
pub trait SessionSource: Send + Sync {
    /// Fetch the recorded turns for a session, in turn order.
    async fn fetch_turns(&self, session: SessionId) -> ArgoResult<Vec<TurnRecord>>;
}

/// Test-only in-memory source.
#[derive(Debug, Default, Clone)]
pub struct InMemorySource {
    by_session: std::collections::HashMap<SessionId, Vec<TurnRecord>>,
}

impl InMemorySource {
    /// New empty source.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a session's turns.
    pub fn insert(&mut self, session: SessionId, turns: Vec<TurnRecord>) {
        self.by_session.insert(session, turns);
    }
}

#[async_trait]
impl SessionSource for InMemorySource {
    async fn fetch_turns(&self, session: SessionId) -> ArgoResult<Vec<TurnRecord>> {
        self.by_session
            .get(&session)
            .cloned()
            .ok_or_else(|| ArgoError::Schema(format!("unknown session {}", session.0)))
    }
}
