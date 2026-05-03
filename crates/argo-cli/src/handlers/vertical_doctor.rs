//! `argo vertical doctor <id>` (#673) — composite "is this
//! vertical OK?" health check.
//!
//! Composes three existing checks into one report so an
//! operator gets the full picture without running three
//! commands:
//!
//! | Check     | Pass criteria                            |
//! |-----------|------------------------------------------|
//! | validate  | argo_vertical::validate returns Ok(())   |
//! | eval      | every JSONL suite parses (or no suites)  |
//! | lint      | no severity=error findings               |
//!
//! Exit code: 1 if any check fails, 0 otherwise.
//!
//! No new behaviour — this is pure composition. The constituent
//! commands keep their own dedicated CLI surfaces (`argo
//! vertical validate / eval / lint`) and unit tests; doctor
//! just wires them together.
use argo_types::{ArgoError, ArgoResult};

pub async fn run(id: &str, json: bool) -> ArgoResult<()> {
    let dir = super::vertical::resolve_verticals_dir();
    let (registry, _errs) = argo_vertical::load_dir(&dir)?;
    let loaded = registry.get(id).ok_or_else(|| {
        ArgoError::Provider(format!(
            "vertical '{id}' not found in {} — run `argo vertical list` to see what's loaded",
            dir.display()
        ))
    })?;

    // 1. Structural validate
    let validate_err = argo_vertical::validate(&loaded.manifest, &loaded.root)
        .err()
        .map(|e| format!("{e}"));
    let validate_ok = validate_err.is_none();

    // 2. Eval-suite structural check
    let eval_report = super::vertical_eval::collect_eval_report(loaded);

    // 3. Lint
    use super::vertical_lint::{lint_loaded, Severity};
    let lint_findings = lint_loaded(loaded);
    let lint_error_count = lint_findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .count();
    let lint_warn_count = lint_findings
        .iter()
        .filter(|f| f.severity == Severity::Warn)
        .count();
    let lint_info_count = lint_findings
        .iter()
        .filter(|f| f.severity == Severity::Info)
        .count();
    let lint_ok = lint_error_count == 0;

    let any_failed = !validate_ok || !eval_report.ok || !lint_ok;

    if json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "ok": !any_failed,
                "id": id,
                "validate": {
                    "ok": validate_ok,
                    "error": validate_err,
                },
                "eval": eval_report.to_json(),
                "lint": {
                    "ok": lint_ok,
                    "error_count": lint_error_count,
                    "warn_count": lint_warn_count,
                    "info_count": lint_info_count,
                    "findings": lint_findings.iter().map(|f| serde_json::json!({
                        "severity": f.severity.label(),
                        "code": f.code,
                        "message": f.message,
                    })).collect::<Vec<_>>(),
                },
            }))
            .unwrap_or_default()
        );
    } else {
        println!("vertical '{id}' health check:");
        let validate_marker = if validate_ok { "ok" } else { "FAIL" };
        println!("  [{validate_marker}] validate");
        if let Some(e) = &validate_err {
            println!("    error: {e}");
        }
        let eval_marker = if eval_report.ok { "ok" } else { "FAIL" };
        let suite_count = eval_report.suites.len();
        let total_fixtures: usize = eval_report
            .suites
            .iter()
            .map(|s| s.fixture_count)
            .sum();
        println!(
            "  [{eval_marker}] eval ({suite_count} suite(s), {total_fixtures} fixture(s) total)"
        );
        for s in &eval_report.suites {
            for err in &s.errors {
                println!("    {} → {err}", s.path);
            }
        }
        let lint_marker = if lint_ok { "ok" } else { "FAIL" };
        println!(
            "  [{lint_marker}] lint ({lint_error_count} error, {lint_warn_count} warn, {lint_info_count} info)"
        );
        for f in &lint_findings {
            println!("    [{}] {} — {}", f.severity.label(), f.code, f.message);
        }
        let overall = if any_failed { "FAILED" } else { "PASSED" };
        println!();
        println!("overall: {overall}");
    }

    if any_failed {
        return Err(ArgoError::Provider(format!(
            "vertical '{id}' doctor: one or more checks failed"
        )));
    }
    Ok(())
}
