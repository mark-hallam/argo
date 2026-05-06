//! Per-action audit log + screenshot capture. Writes
//! `browser.jsonl` (one JSON line per CU step) and
//! `step-<n>.png` files under
//! `.skycot/outputs/<job_id>/actions/<action_id>/`. The audit
//! trail satisfies the v1 success criterion that every action
//! has a screenshot + evidence chain.

use crate::budget::BudgetSnapshot;
use crate::e2b::{ActionResult, CuAction};
use crate::CuError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// One row in `browser.jsonl`. Logged after every CU step so
/// the trail is complete even if the executor crashes mid-loop.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditRow {
    /// 1-indexed CU step number.
    pub step: u32,
    /// ISO-8601 UTC timestamp.
    pub at: String,
    /// CU action the model emitted.
    pub action: CuAction,
    /// Sandbox's reply.
    pub result: ActionResult,
    /// Path to the screenshot captured AFTER this step,
    /// relative to the action bundle dir.
    pub screenshot_relpath: String,
    /// Tokens used this step.
    pub tokens_used: u32,
    /// Cumulative budget snapshot AFTER this step.
    pub budget: BudgetSnapshot,
}

/// On-disk audit logger for one action. Built once per dispatch
/// and reused across CU steps. Holds the bundle directory + the
/// open `browser.jsonl` file so each row is a single append.
pub struct AuditLog {
    bundle_dir: PathBuf,
    jsonl_path: PathBuf,
}

impl AuditLog {
    /// Create the bundle dir on disk and stamp an empty
    /// `browser.jsonl`. Returns an `AuditLog` that subsequent
    /// `append` calls write to.
    pub fn create(bundle_dir: PathBuf) -> Result<Self, CuError> {
        std::fs::create_dir_all(&bundle_dir).map_err(|e| CuError::Io {
            what: "audit.bundle_dir.create",
            source: e,
        })?;
        let jsonl_path = bundle_dir.join("browser.jsonl");
        // Touch the file so callers can rely on its existence
        // even if the loop terminates without any steps.
        std::fs::write(&jsonl_path, b"").map_err(|e| CuError::Io {
            what: "audit.browser_jsonl.create",
            source: e,
        })?;
        Ok(Self {
            bundle_dir,
            jsonl_path,
        })
    }

    /// Path to the bundle dir. Public so the executor can
    /// surface it in the `Outcome::HumanHandoff` /
    /// `Outcome::Completed` payload.
    #[must_use]
    pub fn bundle_dir(&self) -> &Path {
        &self.bundle_dir
    }

    /// Persist a screenshot for step N. Returns the relative
    /// path (relative to `bundle_dir`) that gets stamped into
    /// the `AuditRow`.
    pub fn save_screenshot(&self, step: u32, png: &[u8]) -> Result<String, CuError> {
        let filename = format!("step-{step:03}.png");
        let path = self.bundle_dir.join(&filename);
        std::fs::write(&path, png).map_err(|e| CuError::Io {
            what: "audit.screenshot.write",
            source: e,
        })?;
        Ok(filename)
    }

    /// Append one row to `browser.jsonl`.
    pub fn append(&self, row: &AuditRow) -> Result<(), CuError> {
        let line = serde_json::to_string(row).map_err(|e| CuError::Json {
            what: "audit.row.serialise",
            source: e,
        })?;
        let mut body = std::fs::read(&self.jsonl_path).map_err(|e| CuError::Io {
            what: "audit.browser_jsonl.read",
            source: e,
        })?;
        body.extend_from_slice(line.as_bytes());
        body.push(b'\n');
        std::fs::write(&self.jsonl_path, &body).map_err(|e| CuError::Io {
            what: "audit.browser_jsonl.append",
            source: e,
        })?;
        Ok(())
    }

    /// Read every row currently in `browser.jsonl`. Used by
    /// tests + by the executor when synthesising the action
    /// trajectory for `result_json`.
    pub fn rows(&self) -> Result<Vec<AuditRow>, CuError> {
        let body = std::fs::read_to_string(&self.jsonl_path).map_err(|e| CuError::Io {
            what: "audit.browser_jsonl.read",
            source: e,
        })?;
        let mut rows = Vec::new();
        for line in body.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let row: AuditRow = serde_json::from_str(line).map_err(|e| CuError::Json {
                what: "audit.row.parse",
                source: e,
            })?;
            rows.push(row);
        }
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_row(step: u32, screenshot: &str) -> AuditRow {
        AuditRow {
            step,
            at: "2026-05-06T12:34:56Z".into(),
            action: CuAction::Click { x: 1, y: 2 },
            result: ActionResult {
                ok: true,
                text: None,
                screenshot_b64: None,
            },
            screenshot_relpath: screenshot.into(),
            tokens_used: 42,
            budget: BudgetSnapshot {
                tokens_used: 42,
                tokens_cap: 100,
                steps_used: step,
                steps_cap: 5,
                wallclock_seconds_used: 1,
                wallclock_seconds_cap: 60,
            },
        }
    }

    #[test]
    fn create_stamps_empty_browser_jsonl() {
        let dir = tempfile::tempdir().unwrap();
        let log = AuditLog::create(dir.path().to_path_buf()).unwrap();
        assert!(log.bundle_dir().join("browser.jsonl").exists());
        assert_eq!(log.rows().unwrap(), Vec::<AuditRow>::new());
    }

    #[test]
    fn append_persists_each_row_and_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let log = AuditLog::create(dir.path().to_path_buf()).unwrap();
        log.append(&sample_row(1, "step-001.png")).unwrap();
        log.append(&sample_row(2, "step-002.png")).unwrap();
        let rows = log.rows().unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].step, 1);
        assert_eq!(rows[0].screenshot_relpath, "step-001.png");
        assert_eq!(rows[1].step, 2);
    }

    #[test]
    fn save_screenshot_writes_step_n_png_with_padded_filename() {
        let dir = tempfile::tempdir().unwrap();
        let log = AuditLog::create(dir.path().to_path_buf()).unwrap();
        let rel = log.save_screenshot(3, b"\x89PNG-fake").unwrap();
        assert_eq!(rel, "step-003.png");
        let path = log.bundle_dir().join("step-003.png");
        assert_eq!(std::fs::read(&path).unwrap(), b"\x89PNG-fake");
    }
}
