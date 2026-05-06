//! Worker config writer. Per the brief: "First 24 hours of any
//! newly-generated worker stay in `dry_run = true`."
//! `WorkerConfig` is a TOML sibling to `vertical.toml`. Stream H
//! reads + enforces `dry_run` + `dry_run_until` at dispatch
//! time; v1 is the writer.

use crate::error::GenesisError;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// 24-hour dry_run window per the brief.
pub const DRY_RUN_HOURS: i64 = 24;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorkerConfig {
    pub worker_id: String,
    pub persona: String,
    /// True until `dry_run_until` passes. While hot, every
    /// emitted action lands `pending` — no executor fires.
    pub dry_run: bool,
    /// ISO-8601 UTC timestamp at which `dry_run` auto-clears.
    pub dry_run_until: String,
    #[serde(default)]
    pub dry_run_reason: String,
}

impl WorkerConfig {
    /// Build a fresh config for a generated worker. Caller passes
    /// `now` so tests can pin it deterministically.
    #[must_use]
    pub fn new_dry_run(worker_id: &str, persona: &str, now: DateTime<Utc>) -> Self {
        let until = now + Duration::hours(DRY_RUN_HOURS);
        Self {
            worker_id: worker_id.to_string(),
            persona: persona.to_string(),
            dry_run: true,
            dry_run_until: until.to_rfc3339(),
            dry_run_reason: format!(
                "auto-set by Genesis at generation time; clears after {DRY_RUN_HOURS}h"
            ),
        }
    }

    /// Write the config to `<dir>/worker.toml`.
    pub fn write_to(&self, dir: &Path) -> Result<(), GenesisError> {
        let body = toml::to_string_pretty(self).map_err(|e| GenesisError::Toml {
            what: "worker_config.serialise",
            source: e,
        })?;
        std::fs::write(dir.join("worker.toml"), body).map_err(|e| GenesisError::Io {
            what: "worker_config.write",
            source: e,
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_dry_run_sets_until_24h_after_now() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-05-06T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let cfg = WorkerConfig::new_dry_run("marketing_intern_maya", "maya", now);
        assert!(cfg.dry_run);
        let until: DateTime<Utc> = cfg.dry_run_until.parse().unwrap();
        let diff = until - now;
        assert_eq!(diff.num_hours(), DRY_RUN_HOURS);
    }

    #[test]
    fn write_to_persists_round_trippable_toml() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = WorkerConfig::new_dry_run("test_worker", "tessa", Utc::now());
        cfg.write_to(dir.path()).unwrap();
        let body = std::fs::read_to_string(dir.path().join("worker.toml")).unwrap();
        let parsed: WorkerConfig = toml::from_str(&body).unwrap();
        assert_eq!(parsed, cfg);
    }

    #[test]
    fn dry_run_reason_explains_auto_set() {
        let cfg = WorkerConfig::new_dry_run("w", "p", Utc::now());
        assert!(cfg.dry_run_reason.contains("Genesis"));
        assert!(cfg.dry_run_reason.contains("24"));
    }
}
