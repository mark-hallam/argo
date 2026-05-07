//! `argo eval list` and shared eval run artifact loading.
use argo_eval::{EvalRunFixture, EvalRunReport};
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct EvalRunListItem {
    pub(crate) id: String,
    pub(crate) vertical: String,
    pub(crate) created_at: String,
    pub(crate) fixture_count: usize,
    pub(crate) pass_rate: f64,
    pub(crate) path: String,
}

pub async fn run(vertical: Option<&str>, limit: usize, json_out: bool) -> ArgoResult<()> {
    let rows = list_runs(vertical, limit)?;
    if let (true, Some(id)) = (rows.is_empty(), vertical) {
        return legacy_fixture_list(id, json_out);
    }
    if json_out {
        return print_json(&json!({"ok": true, "count": rows.len(), "runs": rows}));
    }
    if rows.is_empty() {
        println!("(no eval runs)");
        return Ok(());
    }
    println!("id\tvertical\tcreated_at\tfixtures\tpass_rate");
    for row in rows {
        println!(
            "{}\t{}\t{}\t{}\t{:.0}%",
            row.id,
            row.vertical,
            row.created_at,
            row.fixture_count,
            row.pass_rate * 100.0
        );
    }
    Ok(())
}

pub(crate) fn list_runs(vertical: Option<&str>, limit: usize) -> ArgoResult<Vec<EvalRunListItem>> {
    let dir = runs_dir()?;
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut rows = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let path = entry?.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let run = load_run_path(&path)?;
        if vertical.is_some_and(|want| run.vertical != want) {
            continue;
        }
        let fixture_count = run.fixtures.len();
        let pass_rate = run.pass_rate();
        rows.push(EvalRunListItem {
            id: run.id,
            vertical: run.vertical,
            created_at: run.created_at,
            fixture_count,
            pass_rate,
            path: path.display().to_string(),
        });
    }
    rows.sort_by(|a, b| {
        b.created_at
            .cmp(&a.created_at)
            .then_with(|| b.id.cmp(&a.id))
    });
    rows.truncate(limit);
    Ok(rows)
}

pub(crate) fn load_run_arg(value: &str) -> ArgoResult<EvalRunReport> {
    let direct = PathBuf::from(value);
    if direct.is_file() {
        return load_run_path(&direct);
    }
    let mut path = runs_dir()?.join(value);
    if path.extension().is_none() {
        path.set_extension("json");
    }
    load_run_path(&path)
}

fn load_run_path(path: &Path) -> ArgoResult<EvalRunReport> {
    let body = fs::read_to_string(path)
        .map_err(|e| ArgoError::Provider(format!("read eval run {}: {e}", path.display())))?;
    let value: Value = serde_json::from_str(&body)
        .map_err(|e| ArgoError::Provider(format!("parse eval run {}: {e}", path.display())))?;
    parse_run_value(path, value)
}

fn parse_run_value(path: &Path, value: Value) -> ArgoResult<EvalRunReport> {
    if let Ok(run) = serde_json::from_value::<EvalRunReport>(value.clone()) {
        return Ok(run);
    }
    let id = string_field(&value, "id").unwrap_or_else(|| id_from_path(path));
    let vertical = string_field(&value, "vertical").unwrap_or_else(|| "(unknown)".into());
    let created_at = string_field(&value, "created_at").unwrap_or_else(|| file_created_at(path));
    let fixtures = value
        .get("fixtures")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            ArgoError::Schema(format!(
                "eval run {} must contain a fixtures array",
                path.display()
            ))
        })?
        .iter()
        .map(parse_fixture)
        .collect::<ArgoResult<Vec<_>>>()?;
    Ok(EvalRunReport {
        id,
        vertical,
        created_at,
        fixtures,
    })
}

fn parse_fixture(value: &Value) -> ArgoResult<EvalRunFixture> {
    let fixture = string_field(value, "fixture")
        .or_else(|| string_field(value, "id"))
        .or_else(|| string_field(value, "task"))
        .ok_or_else(|| ArgoError::Schema("eval fixture missing id".into()))?;
    let skill = string_field(value, "skill")
        .or_else(|| {
            string_field(value, "suite").and_then(|s| s.split('/').next().map(str::to_string))
        })
        .unwrap_or_else(|| "(unknown)".into());
    let metric = string_field(value, "metric").unwrap_or_else(|| "fixture".into());
    let passed = bool_field(value, "passed")
        .or_else(|| value.get("score").and_then(|s| bool_field(s, "passed")))
        .unwrap_or(false);
    let score = numeric_score(value).unwrap_or_else(|| f64::from(u8::from(passed)));
    Ok(EvalRunFixture {
        fixture,
        skill,
        metric,
        passed,
        score,
    })
}

fn numeric_score(value: &Value) -> Option<f64> {
    value
        .get("score")
        .and_then(Value::as_f64)
        .or_else(|| {
            value
                .get("score")
                .and_then(|s| s.get("score"))
                .and_then(Value::as_f64)
        })
        .or_else(|| {
            value
                .get("score")
                .and_then(|s| s.get("value"))
                .and_then(Value::as_f64)
        })
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_string)
}

fn bool_field(value: &Value, key: &str) -> Option<bool> {
    value.get(key).and_then(Value::as_bool)
}

fn id_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("eval-run")
        .to_string()
}

fn file_created_at(path: &Path) -> String {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .map(|t| DateTime::<Utc>::from(t).to_rfc3339())
        .unwrap_or_else(|_| "unknown".into())
}

fn runs_dir() -> ArgoResult<PathBuf> {
    if let Some(path) = std::env::var_os("ARGO_EVAL_RUNS_DIR") {
        return Ok(PathBuf::from(path));
    }
    if let Some(path) = std::env::var_os("ARGO_HOME") {
        return Ok(PathBuf::from(path).join("eval-runs"));
    }
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    Ok(cwd.join(".argo").join("eval-runs"))
}

fn legacy_fixture_list(id: &str, json_out: bool) -> ArgoResult<()> {
    let (_registry, loaded) = load_vertical(id)?;
    let fixtures = collect_legacy_fixtures(&loaded)?;
    if json_out {
        let rows: Vec<Value> = fixtures
            .iter()
            .map(|f| {
                json!({
                    "id": f.id,
                    "suite": f.suite,
                    "line": f.line,
                    "input": f.input,
                })
            })
            .collect();
        return print_json(&json!({
            "ok": true,
            "vertical": id,
            "fixture_count": rows.len(),
            "fixtures": rows,
        }));
    }
    println!("vertical '{id}': {} eval fixture(s)", fixtures.len());
    for f in fixtures {
        println!("{}\t{}\t{}", f.id, f.suite, f.input);
    }
    Ok(())
}

fn load_vertical(id: &str) -> ArgoResult<(argo_vertical::Registry, argo_vertical::LoadedVertical)> {
    let dir = super::vertical::resolve_verticals_dir();
    let (registry, errors) = argo_vertical::load_dir(&dir)?;
    let loaded = registry.get(id).cloned().ok_or_else(|| {
        let details = if errors.is_empty() {
            String::new()
        } else {
            let msgs: Vec<String> = errors
                .iter()
                .map(|e| format!("{}: {}", e.dir.display(), e.error))
                .collect();
            format!("; load errors: {}", msgs.join("; "))
        };
        ArgoError::Provider(format!(
            "vertical '{id}' not found in {}{details}",
            dir.display()
        ))
    })?;
    Ok((registry, loaded))
}

#[derive(Clone)]
struct LegacyFixtureRow {
    id: String,
    input: String,
    suite: String,
    line: usize,
}

fn collect_legacy_fixtures(
    loaded: &argo_vertical::LoadedVertical,
) -> ArgoResult<Vec<LegacyFixtureRow>> {
    let suites = loaded
        .manifest
        .evaluation
        .as_ref()
        .map(|e| e.suites.clone())
        .unwrap_or_default();
    let mut rows = Vec::new();
    for rel in suites {
        let path = loaded.root.join(&rel);
        let body = fs::read_to_string(&path)
            .map_err(|e| ArgoError::Provider(format!("read {}: {e}", path.display())))?;
        for (idx, raw) in body.lines().enumerate() {
            let line = idx + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                continue;
            }
            let parsed: Value = serde_json::from_str(trimmed).map_err(|e| {
                ArgoError::Provider(format!("parse {} line {line}: {e}", path.display()))
            })?;
            rows.push(parse_legacy_fixture(&rel, line, parsed)?);
        }
    }
    Ok(rows)
}

fn parse_legacy_fixture(suite: &str, line: usize, raw: Value) -> ArgoResult<LegacyFixtureRow> {
    let id = required_str(&raw, suite, line, "id")?;
    let input = required_str(&raw, suite, line, "input")?;
    Ok(LegacyFixtureRow {
        id,
        input,
        suite: suite.to_string(),
        line,
    })
}

fn required_str(raw: &Value, suite: &str, line: usize, key: &str) -> ArgoResult<String> {
    let value = raw.get(key).and_then(Value::as_str).ok_or_else(|| {
        ArgoError::Schema(format!("{suite} line {line}: '{key}' must be a string"))
    })?;
    if value.is_empty() {
        return Err(ArgoError::Schema(format!(
            "{suite} line {line}: '{key}' must be non-empty"
        )));
    }
    Ok(value.to_string())
}

pub(crate) fn print_json<T: Serialize>(value: &T) -> ArgoResult<()> {
    let body =
        serde_json::to_string(value).map_err(|e| ArgoError::Provider(format!("eval JSON: {e}")))?;
    println!("{body}");
    Ok(())
}
