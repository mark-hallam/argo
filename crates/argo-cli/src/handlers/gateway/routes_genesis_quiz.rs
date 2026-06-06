//! `POST /api/v1/genesis/quiz` — Genesis sandbox comprehension quiz; the
//! "show me what you learned" gate before Hire (Phase 2; memory
//! [[genesis-is-ai-teammate-onboarding-studio]]).
//!
//! A real manager doesn't hand a new hire a stack of SOPs and turn them loose on
//! live tickets — they ask a few questions first, correct a wrong assumption,
//! and re-test. This handler is the answering side of that loop: the to-be-hired
//! teammate runs in a **sandbox** (no production tenant slot, no persistence
//! beyond the Genesis session) and answers the operator's free-form question by
//! retrieving over the documents already ingested via
//! `POST /api/v1/genesis/ingest-document`. It is the HTTP + audit shim around
//! [`argo_genesis::answer_quiz_question`] + [`argo_genesis::read_onboarding_documents`].
//!
//! ## Request (JSON)
//!
//! ```json
//! { "vertical_id": "genesis-sam-ab12cd", "question": "How do I handle a refund over $500?" }
//! ```
//!
//! An empty / omitted `question` "primes" the panel: the response carries the
//! suggested starter questions only (no answer, no audit row).
//!
//! ## No theater
//!
//! The answer is assembled only from the uploaded chunks (no LLM call, no
//! synthesis). When nothing matches — including the empty-memory case — the
//! teammate answers honestly that it has no SOP that covers the question instead
//! of fabricating one. The `answered` flag distinguishes the two paths.
//!
//! ## Tenant scoping (fail-closed)
//!
//! The teammate's directory is resolved with
//! [`super::genesis_tenant::genesis_vertical_dir_for_tenant`] against the
//! caller's OWN write-tenant — the same scoping the ingest route uses — so a
//! `vertical_id` that belongs to a different tenant resolves to 404 and tenant
//! A's quiz can never read tenant B's uploaded knowledge.

use super::body_from_request;
use argo_audit::{AuditKind, AuditSink};
use argo_genesis::{answer_quiz_question, read_onboarding_documents, suggested_quiz_questions};
use argo_store::SessionStore;
use serde::Serialize;
use std::sync::Arc;

/// One cited passage in the JSON response.
#[derive(Debug, Serialize)]
struct CitationJson {
    source_label: String,
    excerpt: String,
}

/// The quiz response surfaced to the SPA.
#[derive(Debug, Serialize)]
struct QuizResponse {
    /// `false` is the honest "no SOP covers this" path — never a fabrication.
    answered: bool,
    answer: String,
    citations: Vec<CitationJson>,
    /// Grounded starter questions the operator can ask with one click.
    suggested_questions: Vec<String>,
}

/// Handle one Genesis sandbox-quiz request.
pub(super) async fn quiz_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let body = body_from_request(req);
    let parsed: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return bad_request(&format!("parse body: {e}")),
    };

    let vertical_id = match parsed.get("vertical_id").and_then(|v| v.as_str()) {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => return bad_request("`vertical_id` (string) is required"),
    };
    let question = parsed
        .get("question")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .unwrap_or("")
        .to_string();
    let role_text = parsed
        .get("role")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Resolve the teammate's directory scoped to the caller's OWN write-tenant —
    // fail-closed on a cross-tenant or unknown id (same gate as ingest).
    let tenant_id = super::routes_genesis_stream_tenant::resolve_write_tenant(store, req).await;
    let verticals_root = argo_vertical::default_dir();
    let Some(vertical_dir) = super::genesis_tenant::genesis_vertical_dir_for_tenant(
        &verticals_root,
        &tenant_id,
        &vertical_id,
    ) else {
        return not_found(&format!(
            "no teammate '{vertical_id}' for this workspace; describe the role first"
        ));
    };

    let documents = read_onboarding_documents(&vertical_dir);
    let suggested = suggested_quiz_questions(&documents, &role_text);

    // Empty question primes the panel with suggestions only — no answer, no audit.
    if question.is_empty() {
        let resp = QuizResponse {
            answered: false,
            answer: String::new(),
            citations: Vec::new(),
            suggested_questions: suggested,
        };
        return ok_json(&resp);
    }

    let answer = answer_quiz_question(&question, &documents);
    emit_audit(
        store,
        req,
        &vertical_id,
        answer.answered,
        answer.citations.len(),
    )
    .await;

    let resp = QuizResponse {
        answered: answer.answered,
        answer: answer.answer,
        citations: answer
            .citations
            .into_iter()
            .map(|c| CitationJson {
                source_label: c.source_label,
                excerpt: c.excerpt,
            })
            .collect(),
        suggested_questions: suggested,
    };
    ok_json(&resp)
}

fn ok_json<T: Serialize>(resp: &T) -> (&'static str, String) {
    (
        "200 OK",
        serde_json::to_string(resp).unwrap_or_else(|_| "{}".to_string()),
    )
}

fn bad_request(message: &str) -> (&'static str, String) {
    ("400 Bad Request", error_envelope("bad_request", message))
}

fn not_found(message: &str) -> (&'static str, String) {
    ("404 Not Found", error_envelope("not_found", message))
}

fn error_envelope(code: &str, message: &str) -> String {
    serde_json::json!({ "error": { "code": code, "message": message } }).to_string()
}

/// Best-effort tenant-scoped audit row — one per answered question. The quiz
/// text itself is NOT written into the chain (only the forensic breadcrumb that
/// a question was asked and whether the uploaded knowledge could answer it). An
/// audit hiccup never fails the quiz response.
async fn emit_audit(
    store: Option<&Arc<SessionStore>>,
    req: &str,
    vertical_id: &str,
    answered: bool,
    citation_count: usize,
) {
    let Some(store) = store else { return };
    let payload = serde_json::json!({
        "endpoint": "genesis/quiz",
        "vertical_id": vertical_id,
        "answered": answered,
        "citation_count": citation_count,
    });
    let principal_id = super::routes_auth_principal::principal_id_for_audit(req);
    let sink = AuditSink::new_with_tenant(Arc::clone(store), super::rate_limit::extract_tenant(req));
    if let Err(e) = sink
        .write(
            None,
            AuditKind::WorkerOnboardingQuizExchange,
            payload,
            principal_id,
        )
        .await
    {
        tracing::warn!(
            target: "gateway::genesis_quiz",
            vertical_id = %vertical_id,
            error = %e,
            "genesis quiz audit write failed"
        );
    }
}

#[cfg(test)]
#[path = "routes_genesis_quiz_tests.rs"]
mod tests;
