//! `GET /verticals/<id>/doctor` (#674) — gateway mirror of
//! `argo vertical doctor <id>`. Same JSON shape as the CLI's
//! `--json` mode.
//!
//! Status semantics:
//! - 200 OK   → all three checks pass
//! - 422 Unprocessable Entity → any check failed (validate /
//!   eval / lint). Body still carries the full report so the
//!   caller can see WHICH check failed without parsing twice.
//! - 404 Not Found → unknown vertical id
//! - 500      → discovery dir unreadable
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

    let any_failed = !validate_ok || !eval_report.ok || !lint_ok;

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
    }))
    .unwrap_or_default();
    let status: &'static str = if any_failed {
        "422 Unprocessable Entity"
    } else {
        "200 OK"
    };
    (status, body)
}
