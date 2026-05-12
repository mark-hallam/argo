//! `POST /api/v1/genesis/{classify,draft-worker}` — HTTP shims
//! around the existing `argo-genesis` pipeline (Stream J4).
//!
//! v1 runs the deterministic keyword classifier; `cost_usd` is
//! reported as `null` until LLM-classifier rollout lands a cost
//! signal. Bearer auth + tenant scoping ride the dispatch layer.

use super::body_from_request;
use argo_audit::{AuditKind, AuditSink};
use argo_genesis::{
    classify_charter, dialogue_to_charter, name_to_persona_slug, plan, CharterDialogue,
    GenesisPlan,
};
use argo_store::SessionStore;
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;

// Forward-looking safety net for LLM rollout — the v1 keyword
// classifier returns in microseconds.
const GENESIS_CALL_TIMEOUT: Duration = Duration::from_secs(30);

// Mirrors the frontend E2-b guard so both layers reject the same.
const MIN_PROBLEM_LEN: usize = 15;

#[derive(Debug, Clone, Serialize)]
struct ClassifyQuestion {
    id: String,
    text: String,
    answer_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<Vec<String>>,
    required: bool,
}

#[derive(Debug, Clone, Serialize)]
struct ClassifyResponse {
    classification: String,
    questions: Vec<ClassifyQuestion>,
    cost_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkerDraft {
    suggested_name: String,
    persona: String,
    charter: String,
    duties: Vec<String>,
    example_replies: Vec<String>,
    approval_policy: String,
    suggested_vertical: String,
}

#[derive(Debug, Clone, Serialize)]
struct DraftResponse {
    worker_draft: WorkerDraft,
    cost_usd: Option<f64>,
}

fn q(id: &str, text: &str, answer_type: &str, options: Option<Vec<&str>>, required: bool) -> ClassifyQuestion {
    ClassifyQuestion {
        id: id.to_string(),
        text: text.to_string(),
        answer_type: answer_type.to_string(),
        options: options.map(|o| o.into_iter().map(String::from).collect()),
        required,
    }
}

fn canonical_questions() -> Vec<ClassifyQuestion> {
    vec![
        q("role", "What role should this worker play? (e.g. marketing intern, customer support, sales SDR)", "free-text", None, true),
        q("name", "What's their name? (one word; becomes the persona slug)", "free-text", None, true),
        q("industry", "Industry? (e.g. B2B SaaS, e-commerce, healthcare — optional)", "free-text", None, false),
        q("duties", "What 3-5 main duties? (one per line)", "free-text", None, true),
        q("channels", "Which channels should this worker use for approvals?", "multi-select",
            Some(vec!["slack-dm", "email", "sms", "discord", "whatsapp"]), true),
        q("spend_cap", "Daily spend cap (USD)?", "free-text", None, true),
    ]
}

pub(super) async fn classify_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let (_, problem) = match parse_problem_envelope(req) {
        Ok(pair) => pair,
        Err(resp) => return resp,
    };

    let work = async {
        ClassifyResponse {
            classification: classify_charter(&problem)
                .map(|a| a.slug().to_string())
                .unwrap_or_else(|_| "unknown".to_string()),
            questions: canonical_questions(),
            cost_usd: None,
        }
    };
    let resp = match tokio::time::timeout(GENESIS_CALL_TIMEOUT, work).await {
        Ok(r) => r,
        Err(_) => return timeout_response(),
    };

    let question_count = resp.questions.len();
    emit_audit(store, req, "classify", problem.len(), question_count, resp.cost_usd).await;
    ok_json(&resp)
}

pub(super) async fn draft_worker_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let (parsed, problem) = match parse_problem_envelope(req) {
        Ok(pair) => pair,
        Err(resp) => return resp,
    };
    let answers = match parsed.get("answers") {
        Some(v) if v.is_object() => v.clone(),
        _ => return bad_request("`answers` (object) is required"),
    };
    let adjustment = parsed.get("adjustment").and_then(|v| v.as_str()).map(str::to_string);
    let dialogue = match build_dialogue(&answers) {
        Ok(d) => d,
        Err(msg) => return bad_request(&msg),
    };

    let work = async {
        let mut charter = dialogue_to_charter(&dialogue);
        if let Some(adj) = adjustment.as_ref() {
            if !adj.trim().is_empty() {
                charter.push_str("\n\nAdjustments:\n");
                charter.push_str(adj.trim());
            }
        }
        let persona = name_to_persona_slug(&dialogue.name);
        plan(&charter, &persona).map(|p| (charter, persona, p))
    };
    let (charter, persona, plan_result) =
        match tokio::time::timeout(GENESIS_CALL_TIMEOUT, work).await {
            Ok(Ok(triple)) => triple,
            Ok(Err(e)) => return server_error(&format!("genesis plan: {e}")),
            Err(_) => return timeout_response(),
        };

    let resp = DraftResponse {
        worker_draft: WorkerDraft {
            suggested_name: dialogue.name.clone(),
            persona,
            charter,
            duties: dialogue.duties.clone(),
            example_replies: derive_example_replies(&plan_result),
            approval_policy: derive_approval_policy(&plan_result),
            suggested_vertical: plan_result.vertical_id.clone(),
        },
        cost_usd: None,
    };
    emit_audit(store, req, "draft-worker", problem.len(), 0, resp.cost_usd).await;
    ok_json(&resp)
}

fn parse_problem_envelope(
    req: &str,
) -> Result<(serde_json::Value, String), (&'static str, String)> {
    let body = body_from_request(req);
    let parsed: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| bad_request(&format!("parse body: {e}")))?;
    let problem = parse_problem(&parsed).map_err(|m| bad_request(&m))?;
    Ok((parsed, problem))
}

fn ok_json<T: Serialize>(value: &T) -> (&'static str, String) {
    (
        "200 OK",
        serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string()),
    )
}

fn parse_problem(parsed: &serde_json::Value) -> Result<String, String> {
    match parsed.get("problem").and_then(|v| v.as_str()) {
        Some(s) if s.trim().len() >= MIN_PROBLEM_LEN => Ok(s.trim().to_string()),
        Some(_) => Err(format!(
            "`problem` must be at least {MIN_PROBLEM_LEN} non-whitespace chars"
        )),
        None => Err("`problem` (string) is required".to_string()),
    }
}

fn build_dialogue(answers: &serde_json::Value) -> Result<CharterDialogue, String> {
    let role = required_str(answers, "role")?;
    let name = required_str(answers, "name")?;
    let industry = answers
        .get("industry")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let duties = required_string_list(answers, "duties")?;
    let channels = required_string_list(answers, "channels")?;
    let spend_cap_per_day_usd = required_f64(answers, "spend_cap")?;
    if !spend_cap_per_day_usd.is_finite() || spend_cap_per_day_usd <= 0.0 {
        return Err("`spend_cap` must be a positive finite number".to_string());
    }
    Ok(CharterDialogue {
        role,
        name,
        industry,
        duties,
        channels,
        spend_cap_per_day_usd,
    })
}

fn required_str(answers: &serde_json::Value, key: &str) -> Result<String, String> {
    match answers.get(key).and_then(|v| v.as_str()) {
        Some(s) if !s.trim().is_empty() => Ok(s.trim().to_string()),
        _ => Err(format!("missing required answer: `{key}`")),
    }
}

// Accept either a JSON array or a newline/comma-delimited string;
// pre-E2-d clients sometimes send the latter form.
fn required_string_list(
    answers: &serde_json::Value,
    key: &str,
) -> Result<Vec<String>, String> {
    let raw = answers
        .get(key)
        .ok_or_else(|| format!("missing required answer: `{key}`"))?;
    let items: Vec<String> = if let Some(arr) = raw.as_array() {
        arr.iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else if let Some(s) = raw.as_str() {
        s.split(['\n', ','])
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty())
            .collect()
    } else {
        return Err(format!(
            "`{key}` must be an array of strings or a delimited string"
        ));
    };
    if items.is_empty() {
        return Err(format!("missing required answer: `{key}`"));
    }
    Ok(items)
}

fn required_f64(answers: &serde_json::Value, key: &str) -> Result<f64, String> {
    let raw = answers
        .get(key)
        .ok_or_else(|| format!("missing required answer: `{key}`"))?;
    if let Some(n) = raw.as_f64() {
        return Ok(n);
    }
    raw.as_str()
        .ok_or_else(|| format!("`{key}` must be a number"))?
        .trim()
        .parse::<f64>()
        .map_err(|_| format!("`{key}` is not a valid number"))
}

fn derive_example_replies(gp: &GenesisPlan) -> Vec<String> {
    gp.manifest
        .actions
        .as_ref()
        .map(|a| {
            a.actions
                .values()
                .filter(|decl| !decl.description.trim().is_empty())
                .take(3)
                .map(|decl| format!("Sure — I can {}.", decl.description.trim()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn derive_approval_policy(gp: &GenesisPlan) -> String {
    let Some(actions) = gp.manifest.actions.as_ref() else {
        return "No actions declared; manifest is informational only.".to_string();
    };
    let total = actions.actions.len();
    if total == 0 {
        return "No actions declared; manifest is informational only.".to_string();
    }
    let gated = actions.actions.values().filter(|d| d.requires_approval).count();
    format!(
        "{gated} of {total} actions require operator approval. Sensitive verbs (send, post, pay) need explicit operator OK before the worker can act."
    )
}

fn bad_request(message: &str) -> (&'static str, String) {
    ("400 Bad Request", error_envelope("bad_request", message))
}

fn server_error(message: &str) -> (&'static str, String) {
    ("500 Internal Server Error", error_envelope("internal", message))
}

fn timeout_response() -> (&'static str, String) {
    (
        "408 Request Timeout",
        error_envelope("timeout", "Genesis is taking longer than usual. Try again."),
    )
}

fn error_envelope(code: &str, message: &str) -> String {
    serde_json::json!({ "error": { "code": code, "message": message } }).to_string()
}

// Best-effort; an audit hiccup never rejects a successful Genesis
// response. The generic `routes_request_audit` wrapper also emits
// a row per request — this one carries the richer J4 metadata.
async fn emit_audit(
    store: Option<&Arc<SessionStore>>,
    req: &str,
    endpoint: &str,
    problem_length: usize,
    question_count: usize,
    cost_usd: Option<f64>,
) {
    let Some(store) = store else { return };
    let payload = serde_json::json!({
        "endpoint": endpoint,
        "problem_length": problem_length,
        "question_count": question_count,
        "cost_usd": cost_usd,
    });
    let principal_id = super::routes_auth_principal::principal_id_for_audit(req);
    let sink = AuditSink::new(Arc::clone(store));
    if let Err(e) = sink
        .write(None, AuditKind::HttpRequest, payload, principal_id)
        .await
    {
        tracing::warn!(
            target: "gateway::genesis",
            endpoint = %endpoint,
            error = %e,
            "genesis audit write failed"
        );
    }
}

#[cfg(test)]
#[path = "routes_genesis_tests.rs"]
mod tests;
