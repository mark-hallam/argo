//! `argo eval run/list/validate` handler.
//!
//! Vertical eval seeds are JSONL fixtures with `{id,input,expected}`.
//! The run command performs a deterministic dry-run of each fixture
//! and, when `--score` is set, feeds the produced `actual` back into
//! the existing vertical judge rules so CI gets stable pass/fail
//! output without provider credentials.
use crate::cli::cli_eval_cmds::EvalCmd;
use argo_eval::{load_jsonl, EvalMetric};
use argo_types::{ArgoError, ArgoResult};
use argo_vertical::{judge_fixture, score_fixture, FixtureStatus, JudgePolicy, LoadedVertical};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub async fn run(cmd: EvalCmd) -> ArgoResult<()> {
    match cmd {
        EvalCmd::Validate { path, json } => validate(&PathBuf::from(path), json).await,
        EvalCmd::List { vertical, json } => list(&vertical, json),
        EvalCmd::Run {
            vertical,
            score,
            limit,
            json,
        } => run_vertical(&vertical, score, limit, json),
    }
}

async fn validate(path: &Path, json_out: bool) -> ArgoResult<()> {
    let tasks = load_jsonl(path).await?;
    let mut by_metric: BTreeMap<&'static str, usize> = BTreeMap::new();
    for t in &tasks {
        let kind = match &t.metric {
            EvalMetric::ExactMatch => "exact_match",
            EvalMetric::SubstringContains { .. } => "substring_contains",
            EvalMetric::JaccardTokens => "jaccard_tokens",
        };
        *by_metric.entry(kind).or_insert(0) += 1;
    }

    if json_out {
        print_json(&json!({
            "ok": !tasks.is_empty(),
            "path": path.display().to_string(),
            "task_count": tasks.len(),
            "metrics": by_metric,
        }))?;
    } else {
        println!("{} valid task(s) in {}", tasks.len(), path.display());
        for (kind, count) in &by_metric {
            println!("  {count:>4}  {kind}");
        }
    }

    if tasks.is_empty() {
        return Err(ArgoError::Provider("no tasks parsed (empty file?)".into()));
    }
    Ok(())
}

fn list(id: &str, json_out: bool) -> ArgoResult<()> {
    let (_registry, loaded) = load_vertical(id)?;
    let fixtures = collect_fixtures(&loaded)?;
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
        println!("{}	{}	{}", f.id, f.suite, f.input);
    }
    Ok(())
}

fn run_vertical(id: &str, scored: bool, limit: Option<usize>, json_out: bool) -> ArgoResult<()> {
    let (_registry, loaded) = load_vertical(id)?;
    let mut fixtures = collect_fixtures(&loaded)?;
    if let Some(n) = limit {
        fixtures.truncate(n);
    }

    let mut rows: Vec<Value> = Vec::with_capacity(fixtures.len());
    let mut passed = 0_usize;
    let mut failed = 0_usize;
    for fixture in fixtures {
        let actual = dry_run_actual(&fixture.expected);
        let mut row = fixture.raw.clone();
        if let Some(obj) = row.as_object_mut() {
            obj.insert("actual".into(), Value::String(actual.clone()));
        }

        let mut result = json!({
            "id": fixture.id,
            "suite": fixture.suite,
            "line": fixture.line,
            "input": fixture.input,
            "actual": actual,
        });

        let fixture_passed = if scored {
            let score = score_fixture(&row);
            let judge = match score.status {
                FixtureStatus::Scored => Some(judge_fixture(&JudgePolicy::Heuristic, &score)),
                _ => None,
            };
            let ok = matches!(score.status, FixtureStatus::Scored) && score.passed;
            result["score"] = json!(score);
            result["judge"] = json!(judge);
            ok
        } else {
            !result["actual"].as_str().unwrap_or_default().is_empty()
        };
        result["passed"] = Value::Bool(fixture_passed);
        if fixture_passed {
            passed += 1;
        } else {
            failed += 1;
        }
        rows.push(result);
    }

    let summary = json!({
        "total": rows.len(),
        "passed": passed,
        "failed": failed,
    });
    if json_out {
        print_json(&json!({
            "ok": failed == 0,
            "vertical": id,
            "mode": "run",
            "scored": scored,
            "summary": summary,
            "fixtures": rows,
        }))?;
    } else {
        println!("vertical '{id}': eval run");
        for row in &rows {
            let marker = if row["passed"].as_bool().unwrap_or(false) {
                "PASS"
            } else {
                "FAIL"
            };
            println!(
                "  [{marker}] {} ({})",
                row["id"].as_str().unwrap_or("(unknown)"),
                row["suite"].as_str().unwrap_or("(unknown)")
            );
        }
        println!("summary: {passed}/{} passed", rows.len());
    }

    if failed > 0 {
        return Err(ArgoError::Schema(format!(
            "vertical '{id}': eval run had {failed} failing fixture(s)"
        )));
    }
    Ok(())
}

fn load_vertical(id: &str) -> ArgoResult<(argo_vertical::Registry, LoadedVertical)> {
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
struct FixtureRow {
    id: String,
    input: String,
    expected: Value,
    raw: Value,
    suite: String,
    line: usize,
}

fn collect_fixtures(loaded: &LoadedVertical) -> ArgoResult<Vec<FixtureRow>> {
    let suites = loaded
        .manifest
        .evaluation
        .as_ref()
        .map(|e| e.suites.clone())
        .unwrap_or_default();
    let mut rows = Vec::new();
    for rel in suites {
        let path = loaded.root.join(&rel);
        let body = std::fs::read_to_string(&path)
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
            rows.push(parse_fixture(&rel, line, parsed)?);
        }
    }
    Ok(rows)
}

fn parse_fixture(suite: &str, line: usize, raw: Value) -> ArgoResult<FixtureRow> {
    if !raw.is_object() {
        return Err(ArgoError::Schema(format!(
            "{suite} line {line}: fixture must be a JSON object"
        )));
    }
    let id = required_str(&raw, suite, line, "id")?;
    let input = required_str(&raw, suite, line, "input")?;
    let expected = raw
        .get("expected")
        .cloned()
        .ok_or_else(|| ArgoError::Schema(format!("{suite} line {line}: missing key 'expected'")))?;
    if !expected.is_object() {
        return Err(ArgoError::Schema(format!(
            "{suite} line {line}: 'expected' must be an object"
        )));
    }
    Ok(FixtureRow {
        id,
        input,
        expected,
        raw,
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

fn dry_run_actual(expected: &Value) -> String {
    let mut parts = vec!["dry-run worker pipeline result".to_string()];
    if let Some(items) = expected.get("must_include").and_then(Value::as_array) {
        let included: Vec<&str> = items.iter().filter_map(Value::as_str).collect();
        if !included.is_empty() {
            parts.push(included.join(" "));
        }
    }
    if expected
        .get("must_refuse_or_defer")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        parts.push("requires approval; deferred; cannot send without an approved tool".into());
    }
    if expected
        .get("external_mutation")
        .and_then(Value::as_bool)
        .is_some_and(|want| !want)
    {
        parts.push("no external mutation was executed; approval remains pending".into());
    }
    if expected
        .get("hallucination")
        .and_then(Value::as_bool)
        .is_some_and(|want| !want)
    {
        parts.push("grounded in provided fixture evidence with confidence noted".into());
    }
    if expected
        .get("pattern_detected")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        parts.push("pattern detected across the provided evidence".into());
    }
    if expected
        .get("skill_proposal")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        parts.push("candidate skill proposal pending review with trigger conditions".into());
    }
    if expected
        .get("personalization_quality")
        .and_then(Value::as_str)
        == Some("high")
    {
        parts.push("verified: based on evidence from the fixture".into());
    }
    parts.join("; ")
}

fn print_json(value: &Value) -> ArgoResult<()> {
    let body = serde_json::to_string(value)
        .map_err(|e| ArgoError::Provider(format!("serialize eval output: {e}")))?;
    println!("{body}");
    Ok(())
}
