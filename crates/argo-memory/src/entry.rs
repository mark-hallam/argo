//! Memory entry types.
use argo_types::{Lock, SessionId, WorkspaceId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// One of the four canonical memory files (§14.1).
///
/// ```
/// use argo_memory::MemoryFile;
/// assert_eq!(MemoryFile::Memory.filename(), "MEMORY.md");
/// assert_eq!(MemoryFile::parse("USER.md"), Some(MemoryFile::User));
/// assert_eq!(MemoryFile::parse("nope.md"), None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MemoryFile {
    /// Free-form persistent memory.
    Memory,
    /// User profile / preferences.
    User,
    /// Voice + persona.
    Soul,
    /// Per-agent persona overlay.
    Agents,
}

impl MemoryFile {
    /// On-disk filename.
    #[must_use]
    pub fn filename(self) -> &'static str {
        match self {
            Self::Memory => "MEMORY.md",
            Self::User => "USER.md",
            Self::Soul => "SOUL.md",
            Self::Agents => "AGENTS.md",
        }
    }

    /// Parse from filename.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "MEMORY.md" => Some(Self::Memory),
            "USER.md" => Some(Self::User),
            "SOUL.md" => Some(Self::Soul),
            "AGENTS.md" => Some(Self::Agents),
            _ => None,
        }
    }
}

/// One row in `memory_entry`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Stable id.
    pub id: Uuid,
    /// Owning workspace.
    pub workspace_id: WorkspaceId,
    /// Which canonical file this lives in.
    pub file: MemoryFile,
    /// Body text.
    pub body: String,
    /// Session that produced this entry, if any.
    pub learned_in: Option<SessionId>,
    /// When the entry was written.
    pub learned_at: DateTime<Utc>,
    /// If superseded by a later entry, the later entry's id.
    pub superseded_by: Option<Uuid>,
    /// SHA-256 of the body.
    pub hash: [u8; 32],
    /// Edit-protection lock (None if no lock row exists).
    pub lock: Lock,
    /// Vertical that owns this entry, if any (#680, Rule 7).
    /// NULL for legacy / workspace-global entries — those stay
    /// reachable via the existing `append` / `list_active`
    /// paths for backward compat. Vertical-bound writes via
    /// `append_for_vertical` populate this; vertical-scoped
    /// reads via `list_for_vertical` filter by it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical_id: Option<String>,
}

/// Caller-supplied override flag for agent-driven writes against a
/// `Lock::UserProtected` row. The agent loop never constructs `Force` on
/// its own — it requires an explicit human-in-the-loop confirmation,
/// e.g. `argo memory edit --force` or a Slack approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrincipalOverride {
    /// Standard agent dispatch — locks honoured.
    Agent,
    /// Human-in-the-loop override — `UserProtected` is permitted to
    /// write through.
    Force,
}
