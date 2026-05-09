//! Argo-wide error and result types.
use crate::budget::BudgetKind;
use crate::capability::Capability;

/// Argo-wide result.
pub type ArgoResult<T> = Result<T, ArgoError>;

/// Argo-wide error.
#[derive(Debug, thiserror::Error)]
pub enum ArgoError {
    /// A capability check failed.
    #[error("capability denied: {0:?}")]
    CapabilityDenied(Capability),
    /// A budget was exhausted.
    #[error("budget exhausted: {kind:?}")]
    BudgetExhausted {
        /// Which budget.
        kind: BudgetKind,
    },
    /// A provider call failed.
    #[error("provider error: {0}")]
    Provider(String),
    /// A tool call failed.
    #[error("tool error: {tool}: {msg}")]
    Tool {
        /// Tool name.
        tool: String,
        /// Reason.
        msg: String,
    },
    /// I/O error.
    #[error("io error: {0}")]
    Io(String),
    /// Schema validation failed.
    #[error("schema validation failed: {0}")]
    Schema(String),
    /// Internal invariant violated.
    #[error("invariant violated: {0}")]
    Invariant(&'static str),
    /// Operation was interrupted.
    #[error("interrupted")]
    Interrupted,
    /// Attempted to edit a user-protected entity.
    #[error("user-protected: {entity}")]
    UserProtected {
        /// What's protected.
        entity: String,
    },
    /// The judge rejected.
    #[error("judge rejected: {reason}")]
    JudgeRejected {
        /// Why.
        reason: String,
    },
    /// Routing failed.
    #[error("routing failed: {reason}")]
    RoutingFailed {
        /// Why.
        reason: String,
    },
    /// RBAC role lacks the required capability.
    #[error("rbac denied: {role} lacks {cap:?}")]
    RbacDenied {
        /// Role name.
        role: String,
        /// Missing capability.
        cap: Capability,
    },
    /// OCI error.
    #[error("oci error: {0}")]
    Oci(String),
    /// The addressed entity does not exist. Maps to HTTP 404 at the
    /// gateway boundary.
    #[error("not found: {0}")]
    NotFound(String),
}

impl From<std::io::Error> for ArgoError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}
