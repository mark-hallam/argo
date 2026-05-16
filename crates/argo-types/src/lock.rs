//! Edit-protection locks (Amendment 3).
use serde::{Deserialize, Serialize};

/// Edit-protection lock on a memory entry or skill.
///
/// # Examples
///
/// ```
/// use argo_types::Lock;
/// assert_eq!(Lock::default(), Lock::None);
/// let json = serde_json::to_string(&Lock::UserProtected).unwrap();
/// assert_eq!(json, "\"user-protected\"");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Lock {
    /// Default; agent may freely edit.
    #[default]
    None,
    /// Agent cannot edit; only `argo skill/memory edit --force` (RBAC: owner).
    UserProtected,
    /// Agent may edit but lineage hash must remain in chain.
    ContentLocked,
}
