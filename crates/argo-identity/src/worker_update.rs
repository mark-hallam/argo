//! Partial-update struct for [`IdentityStore::update_worker`](crate::IdentityStore::update_worker).
//!
//! `WorkerUpdate` is `Default + Clone`: callers set only the fields they want
//! to change and leave the rest as `None`. The store's COALESCE-based UPDATE
//! preserves current values for `None` fields atomically.

use chrono::{DateTime, Utc};

use crate::WorkerLevel;

/// Partial update for a stored [`Worker`](crate::Worker).
///
/// Every field is `Option<T>`; `None` means "leave the current value unchanged".
/// The first revision of this struct exposes only the worker-row columns that
/// do **not** already have a dedicated mutator on
/// [`IdentityStore`](crate::IdentityStore). Future fields land additively.
///
/// ```
/// use argo_identity::{WorkerLevel, WorkerUpdate};
///
/// let updates = WorkerUpdate::default()
///     .with_vertical_id("support")
///     .with_level(WorkerLevel::Junior)
///     .with_trust_grade(7);
/// assert_eq!(updates.vertical_id.as_deref(), Some("support"));
/// assert_eq!(updates.level, Some(WorkerLevel::Junior));
/// assert_eq!(updates.trust_grade, Some(7));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkerUpdate {
    /// Vertical template id this worker instantiates.
    pub vertical_id: Option<String>,
    /// Tenure level on the promotion ladder.
    pub level: Option<WorkerLevel>,
    /// Routing trust grade (`0` = unknown / fresh row).
    pub trust_grade: Option<u8>,
    /// Most-recent assignment timestamp.
    pub last_used_at: Option<DateTime<Utc>>,
}

impl WorkerUpdate {
    /// Set the [`vertical_id`](Self::vertical_id) field.
    #[must_use]
    pub fn with_vertical_id(mut self, vertical_id: impl Into<String>) -> Self {
        self.vertical_id = Some(vertical_id.into());
        self
    }

    /// Set the [`level`](Self::level) field.
    #[must_use]
    pub fn with_level(mut self, level: WorkerLevel) -> Self {
        self.level = Some(level);
        self
    }

    /// Set the [`trust_grade`](Self::trust_grade) field.
    #[must_use]
    pub fn with_trust_grade(mut self, trust_grade: u8) -> Self {
        self.trust_grade = Some(trust_grade);
        self
    }

    /// Set the [`last_used_at`](Self::last_used_at) field.
    #[must_use]
    pub fn with_last_used_at(mut self, last_used_at: DateTime<Utc>) -> Self {
        self.last_used_at = Some(last_used_at);
        self
    }

    /// `true` when every field is `None` — the store treats this as a no-op
    /// (re-reads and returns the current row without issuing an UPDATE).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.vertical_id.is_none()
            && self.level.is_none()
            && self.trust_grade.is_none()
            && self.last_used_at.is_none()
    }
}
