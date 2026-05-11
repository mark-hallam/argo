//! Small orchestration layer plus JSON step log.

use crate::{WindowsComputerUse, WindowsCuResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Session options for the POC example path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionOptions {
    /// Screenshot output path.
    pub screenshot_path: PathBuf,
    /// JSON step-log output path.
    pub log_path: PathBuf,
}

impl Default for SessionOptions {
    fn default() -> Self {
        let dir = std::env::temp_dir();
        Self {
            screenshot_path: dir.join("argo-windows-cu-screenshot.png"),
            log_path: dir.join("argo-windows-cu-steps.json"),
        }
    }
}

/// Stateful POC session.
#[derive(Debug, Clone)]
pub struct Session {
    computer: WindowsComputerUse,
    opts: SessionOptions,
}

/// One logged step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepRecord {
    /// 0-based step index.
    pub step: u32,
    /// Action label.
    pub action: String,
    /// Result detail.
    pub detail: String,
}

/// Full step log written by [`Session::run_example`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepLog {
    /// Steps in execution order.
    pub steps: Vec<StepRecord>,
}

impl Session {
    /// Construct a session.
    #[must_use]
    pub fn new(computer: WindowsComputerUse, opts: SessionOptions) -> Self {
        Self { computer, opts }
    }

    /// Run the acceptance example: screenshot, click `(100, 100)`,
    /// type `hello`, and write a JSON step log.
    pub fn run_example(&self) -> WindowsCuResult<StepLog> {
        let shot = self.computer.screenshot(&self.opts.screenshot_path)?;
        let click = self.computer.click(100, 100)?;
        let typed = self.computer.r#type("hello")?;
        let log = StepLog {
            steps: vec![
                StepRecord {
                    step: 0,
                    action: "screenshot".into(),
                    detail: shot.path.display().to_string(),
                },
                StepRecord {
                    step: 1,
                    action: click.action,
                    detail: click.detail,
                },
                StepRecord {
                    step: 2,
                    action: typed.action,
                    detail: typed.detail,
                },
            ],
        };
        let body = serde_json::to_vec_pretty(&log)
            .map_err(|e| crate::WindowsCuError::Protocol(format!("serialize step log: {e}")))?;
        std::fs::write(&self.opts.log_path, body)?;
        Ok(log)
    }

    /// Session options.
    #[must_use]
    pub fn options(&self) -> &SessionOptions {
        &self.opts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_paths_have_expected_suffixes() {
        let opts = SessionOptions::default();
        assert!(opts
            .screenshot_path
            .ends_with("argo-windows-cu-screenshot.png"));
        assert!(opts.log_path.ends_with("argo-windows-cu-steps.json"));
    }

    #[test]
    fn step_log_serializes() {
        let log = StepLog {
            steps: vec![StepRecord {
                step: 0,
                action: "screenshot".into(),
                detail: "C:/temp/a.png".into(),
            }],
        };
        let body = serde_json::to_string(&log).unwrap();
        assert!(body.contains("screenshot"));
    }

    #[test]
    fn imports_are_used_for_public_surface() {
        let _ = crate::input::click_args(1, 2);
        let _ = crate::screenshot::powershell_args(std::path::Path::new("C:/x.png"));
    }
}
