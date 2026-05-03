//! `GET /verticals/<id>/doctor` (#674) — gateway mirror of
//! `argo vertical doctor <id>`. Same JSON shape as the CLI's
//! `--json` mode.
//!
//! Status semantics:
//! - 200 OK   → all four checks pass (or scored_eval skipped)
//! - 422 Unprocessable Entity → any check failed (validate /
//!   eval / lint / scored_eval). Body still carries the full
//!   report so the caller can see WHICH check failed without
//!   parsing twice.
//! - 404 Not Found → unknown vertical id
//! - 500      → discovery dir unreadable
//!
//! #696: scored_eval auto-detection ported from the CLI doctor
//! (#695). Reuses the shared `collect_scored_outcomes` helper so
//! both surfaces apply the same convention (look for
//! `eval/known_good_actuals.jsonl` next to each suite); same
//! aggregate `passed | skipped | failed` semantics.
use crate::handlers::vertical_doctor::{collect_scored_outcomes, ScoredStatus};
use crate::handlers::vertical_eval::collect_eval_report;
use crate::handlers::vertical_lint::{lint_loaded, Severity};

pub(super) async fn vertical_doctor_response(id: &str) -> (&'static str, String) {
    let dir = crate::handlers::vertical::resolve_verticals_dir();
    let (registry, _errs) = match argo_vertical::load_dir(&dir) {
        Ok(p) => p,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"verticals load_dir: {e}\"}}"),
            );
        }
    };
    let Some(loaded) = registry.get(id) else {
        return (
            "404 Not Found",
            format!(
                "{{\"error\":\"vertical '{id}' not found in {}\"}}",
                dir.display()
            ),
        );
    };

    let validate_err = argo_vertical::validate(&loaded.manifest, &loaded.root)
        .err()
        .map(|e| format!("{e}"));
    let validate_ok = validate_err.is_none();

    let eval_report = collect_eval_report(loaded);

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

    // #696: fourth check, mirrored from CLI #695. Skipped is
    // OK; failed flips overall to 422 just like the CLI flips
    // to non-zero exit.
    let scored = collect_scored_outcomes(loaded);
    let scored_ok = !matches!(scored.status, ScoredStatus::Failed);

    let any_failed = !validate_ok || !eval_report.ok || !lint_ok || !scored_ok;

    let body = serde_json::to_string(&serde_json::json!({
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
        "scored_eval": scored.to_json(),
    }))
    .unwrap_or_default();
    let status: &'static str = if any_failed {
        "422 Unprocessable Entity"
    } else {
        "200 OK"
    };
    (status, body)
}
