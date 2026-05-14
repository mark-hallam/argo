//! `argo trajectory export` — convert action bundles under
//! `.skycot/outputs/` into fine-tuning-friendly transcripts.
//!
//! Trajectory model (from disk):
//!   `.skycot/outputs/<job_id>/actions/<action_id>/`
//!     ├── action.json     — `ActionRecord` snapshot (action_type,
//!     │                     payload, state, created_at, …)
//!     ├── browser.jsonl   — CU-only; one row per CU step
//!     │                     (action + result + screenshot path)
//!     └── step-NNN.png    — per-step screenshots (CU only)
//!
//! Output formats follow the convention the major fine-tuning
//! pipelines accept:
//!
//!   * `anthropic` — `[{role: "user"|"assistant", content,
//!                       screenshot?}, …]`
//!   * `openai`    — `[{role: "user"|"assistant", content,
//!                       tool_calls?}, …]`
//!   * `jsonl`     — one raw `Trajectory` JSON object per line
//!
//! Filters apply BEFORE format conversion so `--filter passed`
//! doesn't waste cycles formatting trajectories the caller will
//! drop. PII redaction (when enabled) is applied to every text
//! field after format conversion so the same regex covers tool
//! arguments + assistant content.

use super::trajectory_export_render::{base64_encode, render, sha256_hex};
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, NaiveDate, Utc};
use std::fs;
use std::path::{Path, PathBuf};

/// Caller-supplied options. Mirrors the CLI surface 1-to-1 so the
/// dispatch shim is a one-liner.
#[derive(Debug, Clone)]
pub(crate) struct ExportArgs {
    pub since: Option<String>,
    pub until: Option<String>,
    pub format: String,
    pub filter: String,
    pub include_screenshots: bool,
    pub gdpr_redact_emails: bool,
    pub output: Option<PathBuf>,
    pub json: bool,
}

/// Internal state derived from `ExportArgs` after validation.
#[derive(Debug, Clone)]
pub(crate) struct Resolved {
    pub since_micros: Option<i64>,
    pub until_micros: Option<i64>,
    pub format: Format,
    pub filter: Filter,
    pub include_screenshots: bool,
    pub gdpr_redact_emails: bool,
    pub output: Option<PathBuf>,
    pub json: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Format {
    Anthropic,
    Openai,
    Jsonl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Filter {
    Passed,
    All,
}

/// One step in a trajectory. Mirrors the CU `browser.jsonl`
/// row shape, but extended so non-CU actions can carry a
/// human-handoff "step" that just declares the payload + result.
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub(crate) struct TrajectoryStep {
    pub at: String,
    pub action: serde_json::Value,
    pub result: serde_json::Value,
    /// Inline base64 PNG when `include_screenshots = true`,
    /// otherwise the relpath under the bundle dir.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshot: Option<String>,
}

/// One trajectory = one action's full execution trace.
#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct Trajectory {
    pub action_id: String,
    pub action_type: String,
    pub job_id: String,
    pub state: String,
    pub created_at: String,
    pub steps: Vec<TrajectoryStep>,
}

/// Public entry point. Walks the workspace, filters, formats,
/// emits.
pub(crate) async fn run(args: ExportArgs) -> ArgoResult<()> {
    let resolved = Resolved::from_args(args)?;
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let bundles_root = cwd.join(".skycot").join("outputs");
    let trajectories = collect_trajectories(&bundles_root, &resolved)?;
    let bytes = render(&trajectories, &resolved)?;
    let sha = sha256_hex(&bytes);
    if let Some(ref path) = resolved.output {
        fs::write(path, &bytes)
            .map_err(|e| ArgoError::Io(format!("write {}: {e}", path.display())))?;
    } else if !resolved.json {
        std::io::Write::write_all(&mut std::io::stdout(), &bytes)
            .map_err(|e| ArgoError::Provider(format!("stdout write: {e}")))?;
    }
    if resolved.json {
        let envelope = serde_json::json!({
            "rows": trajectories.len(),
            "format": resolved.format,
            "filter": resolved.filter,
            "include_screenshots": resolved.include_screenshots,
            "gdpr_redact_emails": resolved.gdpr_redact_emails,
            "output": resolved.output.as_ref().map(|p| p.display().to_string()),
            "sha256": sha,
            "bytes": bytes.len(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&envelope)
                .unwrap_or_else(|_| "{}".into())
        );
    }
    Ok(())
}

impl Resolved {
    pub(crate) fn from_args(args: ExportArgs) -> ArgoResult<Self> {
        let format = match args.format.as_str() {
            "anthropic" => Format::Anthropic,
            "openai" => Format::Openai,
            "jsonl" => Format::Jsonl,
            other => {
                return Err(ArgoError::Provider(format!(
                    "trajectory export format '{other}' invalid — \
                     valid: anthropic | openai | jsonl"
                )))
            }
        };
        let filter = match args.filter.as_str() {
            "passed" => Filter::Passed,
            "all" => Filter::All,
            other => {
                return Err(ArgoError::Provider(format!(
                    "trajectory export filter '{other}' invalid — \
                     valid: passed | all"
                )))
            }
        };
        let since_micros = match args.since {
            Some(ref raw) => Some(parse_bound(raw, false, "--since")?),
            None => None,
        };
        let until_micros = match args.until {
            Some(ref raw) => Some(parse_bound(raw, true, "--until")?),
            None => None,
        };
        Ok(Self {
            since_micros,
            until_micros,
            format,
            filter,
            include_screenshots: args.include_screenshots,
            gdpr_redact_emails: args.gdpr_redact_emails,
            output: args.output,
            json: args.json,
        })
    }
}

/// Walk `<root>/<job_id>/actions/<action_id>/` and load every
/// trajectory that survives the date + state filters.
pub(crate) fn collect_trajectories(
    bundles_root: &Path,
    resolved: &Resolved,
) -> ArgoResult<Vec<Trajectory>> {
    let mut out = Vec::new();
    let Ok(jobs) = fs::read_dir(bundles_root) else {
        // Empty workspace — return zero trajectories rather than
        // erroring; the caller handles `rows = 0` cleanly.
        return Ok(out);
    };
    for job in jobs.flatten() {
        let actions_dir = job.path().join("actions");
        let Ok(actions) = fs::read_dir(&actions_dir) else {
            continue;
        };
        for action in actions.flatten() {
            let bundle = action.path();
            let trajectory = match load_trajectory(&bundle, resolved.include_screenshots) {
                Ok(t) => t,
                Err(_) => continue, // partial bundle — skip
            };
            if !passes_filters(&trajectory, resolved) {
                continue;
            }
            out.push(trajectory);
        }
    }
    out.sort_by(|a, b| a.created_at.cmp(&b.created_at).then(a.action_id.cmp(&b.action_id)));
    Ok(out)
}

/// Read one bundle into a [`Trajectory`]. Returns Err if the
/// action.json is missing/invalid; partial bundles are silently
/// skipped by the walker.
pub(crate) fn load_trajectory(bundle: &Path, include_screenshots: bool) -> ArgoResult<Trajectory> {
    let action_json_path = bundle.join("action.json");
    let raw = fs::read_to_string(&action_json_path)
        .map_err(|e| ArgoError::Io(format!("read {}: {e}", action_json_path.display())))?;
    let record: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| ArgoError::Provider(format!("parse action.json: {e}")))?;
    let action_id = record
        .get("action_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let action_type = record
        .get("action_type")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let job_id = record
        .get("job_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let state = record
        .get("state")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let created_at = record
        .get("created_at")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let payload = record.get("payload").cloned().unwrap_or(serde_json::Value::Null);
    let result = record.get("result").cloned().unwrap_or(serde_json::Value::Null);
    let mut steps = load_browser_steps(bundle, include_screenshots).unwrap_or_default();
    // Always synthesize a leading "request" step so non-CU
    // trajectories aren't empty — the model sees the action_type
    // + payload as the user turn, the result as the assistant turn.
    let synthesized = TrajectoryStep {
        at: created_at.clone(),
        action: serde_json::json!({
            "kind": "action_request",
            "action_type": action_type,
            "payload": payload,
        }),
        result: serde_json::json!({
            "kind": "action_result",
            "state": state,
            "result": result,
        }),
        screenshot: None,
    };
    steps.insert(0, synthesized);
    Ok(Trajectory {
        action_id,
        action_type,
        job_id,
        state,
        created_at,
        steps,
    })
}

/// Read `browser.jsonl` (CU only). Missing file → empty vec.
fn load_browser_steps(
    bundle: &Path,
    include_screenshots: bool,
) -> ArgoResult<Vec<TrajectoryStep>> {
    let jsonl = bundle.join("browser.jsonl");
    let Ok(raw) = fs::read_to_string(&jsonl) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for line in raw.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let row: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let at = row.get("at").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let action = row.get("action").cloned().unwrap_or(serde_json::Value::Null);
        let result = row.get("result").cloned().unwrap_or(serde_json::Value::Null);
        let screenshot_relpath = row
            .get("screenshot_relpath")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let screenshot = resolve_screenshot(bundle, screenshot_relpath, include_screenshots);
        out.push(TrajectoryStep {
            at,
            action,
            result,
            screenshot,
        });
    }
    Ok(out)
}

fn resolve_screenshot(
    bundle: &Path,
    relpath: &str,
    include_screenshots: bool,
) -> Option<String> {
    if relpath.is_empty() {
        return None;
    }
    if !include_screenshots {
        return Some(relpath.to_string());
    }
    let path = bundle.join(relpath);
    let bytes = fs::read(&path).ok()?;
    Some(format!("data:image/png;base64,{}", base64_encode(&bytes)))
}

/// Apply date + state filters. Pure so the unit tests can drive
/// it directly without staging a real bundle.
pub(crate) fn passes_filters(t: &Trajectory, resolved: &Resolved) -> bool {
    if resolved.filter == Filter::Passed && t.state != "completed" {
        return false;
    }
    let micros = parse_iso_to_micros(&t.created_at);
    if let (Some(s), Some(m)) = (resolved.since_micros, micros) {
        if m < s {
            return false;
        }
    }
    if let (Some(u), Some(m)) = (resolved.until_micros, micros) {
        if m > u {
            return false;
        }
    }
    true
}

fn parse_iso_to_micros(s: &str) -> Option<i64> {
    if s.is_empty() {
        return None;
    }
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.timestamp_micros())
}

fn parse_bound(raw: &str, until: bool, flag: &str) -> ArgoResult<i64> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(raw) {
        return Ok(dt.timestamp_micros());
    }
    let date = NaiveDate::parse_from_str(raw, "%Y-%m-%d").map_err(|e| {
        ArgoError::Provider(format!(
            "invalid {flag} '{raw}': {e} (expected YYYY-MM-DD or RFC3339)"
        ))
    })?;
    let time = if until {
        date.and_hms_micro_opt(23, 59, 59, 999_999)
    } else {
        date.and_hms_micro_opt(0, 0, 0, 0)
    }
    .ok_or_else(|| ArgoError::Schema(format!("invalid {flag} '{raw}'")))?;
    Ok(DateTime::<Utc>::from_naive_utc_and_offset(time, Utc).timestamp_micros())
}

#[cfg(test)]
#[path = "trajectory_export_tests.rs"]
mod tests;
