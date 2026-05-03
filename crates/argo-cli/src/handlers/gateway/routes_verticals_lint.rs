//! `GET /verticals/<id>/lint` (#672) — gateway mirror of
//! `argo vertical lint <id>`. Same JSON shape as the CLI's
//! `--json` mode so scripting clients can parse one document
//! regardless of where the call originated.
//!
//! Response (200 OK on no error findings, 422 Unprocessable
//! Entity when at least one finding has severity=error so HTTP
//! gates can fail loud without parsing the body):
//!
//! ```json
//! {
//!   "ok": true,
//!   "id": "demo",
//!   "findings": [
//!     { "severity": "warn", "code": "tools.allowed.unknown_name", "message": "..." },
//!     { "severity": "info", "code": "skills.bundle_empty",         "message": "..." }
//!   ]
//! }
//! ```
use crate::handlers::vertical_lint::{lint_loaded, Severity};

pub(super) async fn vertical_lint_response(id: &str) -> (&'static str, String) {
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
    let findings = lint_loaded(loaded);
    let any_error = findings.iter().any(|f| f.severity == Severity::Error);
    let arr: Vec<serde_json::Value> = findings
        .iter()
        .map(|f| {
            serde_json::json!({
                "severity": f.severity.label(),
                "code": f.code,
                "message": f.message,
            })
        })
        .collect();
    let body = serde_json::to_string(&serde_json::json!({
        "ok": !any_error,
        "id": id,
        "findings": arr,
    }))
    .unwrap_or_default();
    let status: &'static str = if any_error {
        "422 Unprocessable Entity"
    } else {
        "200 OK"
    };
    (status, body)
}
