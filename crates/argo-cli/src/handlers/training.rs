//! `argo training export` — fine-tuning JSONL from successful action bundles.

use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, NaiveDate, Utc};
use clap::Subcommand;
use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Subcommand, Debug)]
pub enum TrainingCmd {
    /// Export completed action trajectories for one vertical.
    Export {
        /// Vertical id to export.
        #[arg(long)]
        vertical: String,
        /// Inclusive lower bound: YYYY-MM-DD or RFC3339.
        #[arg(long = "start-date")]
        start_date: String,
        /// Output format. Only jsonl is supported.
        #[arg(long, default_value = "jsonl")]
        format: String,
        /// Write JSONL to this path instead of stdout.
        #[arg(long)]
        output: Option<PathBuf>,
        /// Print an export summary envelope instead of the JSONL bytes.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Clone)]
struct ExportArgs {
    vertical: String,
    start_micros: i64,
    output: Option<PathBuf>,
    json: bool,
}

#[derive(Debug, Serialize)]
struct TrainingExample {
    instruction: String,
    input: Value,
    expected_output: Value,
    metadata: TrainingMetadata,
}

#[derive(Debug, Serialize)]
struct TrainingMetadata {
    vertical_id: String,
    job_id: String,
    action_id: String,
    action_type: String,
    created_at: String,
}

pub async fn run(cmd: TrainingCmd) -> ArgoResult<()> {
    let args = match cmd {
        TrainingCmd::Export {
            vertical,
            start_date,
            format,
            output,
            json,
        } => {
            if format != "jsonl" {
                return Err(ArgoError::Provider(format!(
                    "training export format '{format}' invalid - valid: jsonl"
                )));
            }
            ExportArgs {
                vertical,
                start_micros: parse_start_date(&start_date)?,
                output,
                json,
            }
        }
    };
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let root = cwd.join(".skycot").join("outputs");
    let examples = collect_examples(&root, &args)?;
    let body = render_jsonl(&examples)?;
    if let Some(path) = &args.output {
        fs::write(path, &body)
            .map_err(|e| ArgoError::Io(format!("write {}: {e}", path.display())))?;
    } else if !args.json {
        std::io::stdout()
            .write_all(&body)
            .map_err(|e| ArgoError::Provider(format!("stdout write: {e}")))?;
    }
    if args.json {
        println!(
            "{}",
            serde_json::json!({
                "rows": examples.len(),
                "format": "jsonl",
                "vertical": args.vertical,
                "output": args.output.as_ref().map(|p| p.display().to_string()),
                "bytes": body.len(),
            })
        );
    }
    Ok(())
}

fn collect_examples(root: &Path, args: &ExportArgs) -> ArgoResult<Vec<TrainingExample>> {
    let mut out = Vec::new();
    let Ok(jobs) = fs::read_dir(root) else {
        return Ok(out);
    };
    for job in jobs.flatten() {
        let job_dir = job.path();
        let vertical = job_vertical(&job_dir);
        if vertical.as_deref() != Some(args.vertical.as_str()) {
            continue;
        }
        let actions_dir = job_dir.join("actions");
        let Ok(actions) = fs::read_dir(actions_dir) else {
            continue;
        };
        for action in actions.flatten() {
            if let Some(example) =
                load_action_example(&action.path(), &args.vertical, args.start_micros)?
            {
                out.push(example);
            }
        }
    }
    out.sort_by(|a, b| {
        a.metadata
            .created_at
            .cmp(&b.metadata.created_at)
            .then(a.metadata.action_id.cmp(&b.metadata.action_id))
    });
    Ok(out)
}

fn job_vertical(job_dir: &Path) -> Option<String> {
    let raw = fs::read_to_string(job_dir.join("job.json")).ok()?;
    let json: Value = serde_json::from_str(&raw).ok()?;
    json.get("vertical_id")
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn load_action_example(
    action_dir: &Path,
    vertical: &str,
    start_micros: i64,
) -> ArgoResult<Option<TrainingExample>> {
    let raw = match fs::read_to_string(action_dir.join("action.json")) {
        Ok(raw) => raw,
        Err(_) => return Ok(None),
    };
    let action: Value = serde_json::from_str(&raw)
        .map_err(|e| ArgoError::Provider(format!("parse action.json: {e}")))?;
    if action.get("state").and_then(Value::as_str) != Some("completed") {
        return Ok(None);
    }
    let created_at = action
        .get("created_at")
        .and_then(Value::as_str)
        .unwrap_or("");
    if parse_created_at(created_at).is_some_and(|micros| micros < start_micros) {
        return Ok(None);
    }
    let action_type = string_field(&action, "action_type");
    let action_id = string_field(&action, "action_id");
    let job_id = string_field(&action, "job_id");
    Ok(Some(TrainingExample {
        instruction: format!("Perform the `{action_type}` action for the `{vertical}` vertical."),
        input: serde_json::json!({
            "action_type": action_type,
            "payload": action.get("payload").cloned().unwrap_or(Value::Null),
        }),
        expected_output: action.get("result").cloned().unwrap_or(Value::Null),
        metadata: TrainingMetadata {
            vertical_id: vertical.to_string(),
            job_id,
            action_id,
            action_type,
            created_at: created_at.to_string(),
        },
    }))
}

fn render_jsonl(examples: &[TrainingExample]) -> ArgoResult<Vec<u8>> {
    let mut out = Vec::new();
    for example in examples {
        serde_json::to_writer(&mut out, example)
            .map_err(|e| ArgoError::Provider(format!("training jsonl serialize: {e}")))?;
        out.push(b'\n');
    }
    Ok(out)
}

fn string_field(json: &Value, key: &str) -> String {
    json.get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn parse_start_date(raw: &str) -> ArgoResult<i64> {
    if let Some(micros) = parse_created_at(raw) {
        return Ok(micros);
    }
    let date = NaiveDate::parse_from_str(raw, "%Y-%m-%d").map_err(|e| {
        ArgoError::Provider(format!(
            "invalid --start-date '{raw}': {e} (expected YYYY-MM-DD or RFC3339)"
        ))
    })?;
    let time = date
        .and_hms_micro_opt(0, 0, 0, 0)
        .ok_or_else(|| ArgoError::Schema(format!("invalid --start-date '{raw}'")))?;
    Ok(DateTime::<Utc>::from_naive_utc_and_offset(time, Utc).timestamp_micros())
}

fn parse_created_at(raw: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt| dt.timestamp_micros())
}

#[cfg(test)]
#[path = "training_tests.rs"]
mod tests;
