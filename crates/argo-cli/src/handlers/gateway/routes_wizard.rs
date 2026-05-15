//! First-run browser wizard endpoints.

use super::body_from_request;
use crate::handlers::wizard_onboard::HttpKeyVerifier;
use crate::handlers::wizard_onboard_http;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct AnswerBody {
    answer: String,
}

pub(super) async fn dispatch_get(req: &str) -> Option<(&'static str, String)> {
    if req.starts_with("OPTIONS /api/v1/wizard ")
        || req.starts_with("OPTIONS /api/v1/wizard/")
        || req.starts_with("OPTIONS /api/v1/wizard?")
    {
        return Some((with_cors("204 No Content"), String::new()));
    }
    if req.starts_with("GET /api/v1/wizard ") || req.starts_with("GET /api/v1/wizard?") {
        return Some(current_response(req));
    }
    None
}

pub(super) async fn dispatch_post(req: &str) -> Option<(&'static str, String)> {
    if req.starts_with("POST /api/v1/wizard/reset ")
        || req.starts_with("POST /api/v1/wizard/reset?")
    {
        return Some(reset_response(req));
    }
    if req.starts_with("POST /api/v1/wizard/answer ")
        || req.starts_with("POST /api/v1/wizard/answer?")
    {
        return Some(answer_response(req));
    }
    None
}

fn current_response(req: &str) -> (&'static str, String) {
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(e) => return server_error(&format!("current_dir: {e}")),
    };
    let config = wizard_onboard_http::default_config_path(&cwd);
    match wizard_onboard_http::current(tenant_key(req).as_str(), &config) {
        Ok(s) => ok_json(&s),
        Err(e) => server_error(&e.to_string()),
    }
}

fn reset_response(req: &str) -> (&'static str, String) {
    match wizard_onboard_http::reset(tenant_key(req).as_str()) {
        Ok(s) => ok_json(&s),
        Err(e) => server_error(&e.to_string()),
    }
}

fn answer_response(req: &str) -> (&'static str, String) {
    let parsed: AnswerBody = match serde_json::from_str(body_from_request(req)) {
        Ok(v) => v,
        Err(e) => return bad_request(&format!("parse body: {e}")),
    };
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(e) => return server_error(&format!("current_dir: {e}")),
    };
    let config = wizard_onboard_http::default_config_path(&cwd);
    match wizard_onboard_http::submit_answer(
        tenant_key(req).as_str(),
        &parsed.answer,
        &HttpKeyVerifier,
        &cwd,
        &config,
    ) {
        Ok(s) => ok_json(&s),
        Err(e) => bad_request(&e.to_string()),
    }
}

fn tenant_key(req: &str) -> String {
    req.lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("x-argo-tenant-id")
                .then(|| value.trim().to_string())
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "default".to_string())
}

fn ok_json<T: serde::Serialize>(value: &T) -> (&'static str, String) {
    (
        with_cors("200 OK"),
        serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string()),
    )
}

fn bad_request(message: &str) -> (&'static str, String) {
    envelope("400 Bad Request", "bad_request", message)
}

fn server_error(message: &str) -> (&'static str, String) {
    envelope("500 Internal Server Error", "server_error", message)
}

fn envelope(status: &'static str, code: &str, message: &str) -> (&'static str, String) {
    (
        with_cors(status),
        serde_json::json!({ "error": { "code": code, "message": message } }).to_string(),
    )
}

fn with_cors(status: &'static str) -> &'static str {
    match status {
        "200 OK" => "200 OK\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: Content-Type, Authorization, X-Argo-Tenant-Id\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS",
        "204 No Content" => "204 No Content\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: Content-Type, Authorization, X-Argo-Tenant-Id\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS",
        "400 Bad Request" => "400 Bad Request\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: Content-Type, Authorization, X-Argo-Tenant-Id\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS",
        "500 Internal Server Error" => "500 Internal Server Error\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: Content-Type, Authorization, X-Argo-Tenant-Id\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS",
        _ => status,
    }
}
