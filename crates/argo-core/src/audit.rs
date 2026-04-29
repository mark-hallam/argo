//! Audit re-exports — argo-core uses `argo-audit` as the canonical
//! sink (§17). This module is kept as a thin re-export so the
//! Phase-1 walking-skeleton API stays stable.
pub use argo_audit::{AuditKind, AuditSink};
