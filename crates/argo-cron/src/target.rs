//! System-target convention for [`CronTarget`].
//!
//! The existing [`CronTarget`] is a `(platform, chat)` pair that
//! adapters interpret as a delivery channel. The cron-tick
//! maintenance-jobs work (`mission-cron-tick-maintenance-jobs`)
//! introduces a fifth flavour of target — workspace-global system
//! maintenance jobs that don't deliver anywhere, they invoke an
//! internal handler registered with the [`crate::SystemJobRegistry`].
//!
//! Rather than break the existing struct shape (and every consumer
//! in the gateway / worker create / routes-cron paths), system
//! targets reuse the same shape via the reserved values
//! `platform = "system"`, `chat = "<job-name>"`. This module is the
//! parser + builder for that convention plus the canonical job
//! names used by the four shipped system jobs.
//!
//! ```
//! use argo_cron::{system_job_name, system_target, SYSTEM_JOB_FORGE_OPTIMIZE};
//! let target = system_target(SYSTEM_JOB_FORGE_OPTIMIZE);
//! assert_eq!(system_job_name(&target), Some(SYSTEM_JOB_FORGE_OPTIMIZE));
//! ```
use crate::CronTarget;

/// The reserved `CronTarget::platform` value indicating a system
/// maintenance job rather than an output-delivery channel.
pub const SYSTEM_PLATFORM: &str = "system";

/// Canonical system job name: every-6h Forge optimize tick.
pub const SYSTEM_JOB_FORGE_OPTIMIZE: &str = "forge.optimize";

/// Canonical system job name: daily 3am UTC skill-quality recompute.
pub const SYSTEM_JOB_SKILL_QUALITY_RECOMPUTE: &str = "skill.quality_recompute";

/// Canonical system job name: daily 4:30am UTC Dreams curation tick.
pub const SYSTEM_JOB_DREAMS_CURATION: &str = "dreams.curation_tick";

/// Canonical system job name: weekly Sun 5am UTC A/B promotion sweep.
pub const SYSTEM_JOB_AB_PROMOTION: &str = "ab.promotion";

/// The full set of system job names shipped by `argo cron
/// init-system`. Useful for default-registration paths that want to
/// iterate every canonical job.
pub const CANONICAL_SYSTEM_JOBS: &[&str] = &[
    SYSTEM_JOB_FORGE_OPTIMIZE,
    SYSTEM_JOB_SKILL_QUALITY_RECOMPUTE,
    SYSTEM_JOB_DREAMS_CURATION,
    SYSTEM_JOB_AB_PROMOTION,
];

/// Construct a [`CronTarget`] addressing the named system job.
#[must_use]
pub fn system_target(name: impl Into<String>) -> CronTarget {
    CronTarget {
        platform: SYSTEM_PLATFORM.into(),
        chat: name.into(),
    }
}

/// Return the system job name if `target` is a system target, else
/// `None`. The check is the literal `platform == "system"` value;
/// other adapters are free to use any platform string they want and
/// will not be matched by this predicate.
#[must_use]
pub fn system_job_name(target: &CronTarget) -> Option<&str> {
    if target.platform == SYSTEM_PLATFORM {
        Some(target.chat.as_str())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_system_target() {
        let t = system_target(SYSTEM_JOB_FORGE_OPTIMIZE);
        assert_eq!(system_job_name(&t), Some(SYSTEM_JOB_FORGE_OPTIMIZE));
    }

    #[test]
    fn non_system_target_returns_none() {
        let t = CronTarget {
            platform: "telegram".into(),
            chat: "abc".into(),
        };
        assert_eq!(system_job_name(&t), None);
    }

    #[test]
    fn canonical_jobs_are_all_distinct_strings() {
        let mut seen = std::collections::HashSet::new();
        for name in CANONICAL_SYSTEM_JOBS {
            assert!(seen.insert(*name), "duplicate canonical name: {name}");
        }
        assert_eq!(seen.len(), 4);
    }
}
