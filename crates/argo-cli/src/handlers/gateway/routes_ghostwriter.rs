//! `POST /api/v1/skills/draft-from-prompt` — Ghostwriter pipeline
//! gateway entry (Track L brief: `streams/L6-ghostwriter-prompt-to-ir.md`).
//!
//! Operator types a natural-language description; the model returns a
//! structured `SkillGraph`; the graph is persisted via L2 storage; the
//! caller's frontend redirects to the L4 canvas with the new
//! `(skillId, version)` so the canvas is never blank.
//!
//! v1 wires the deterministic [`OfflineGhostwriterClient`] so the
//! endpoint is functional ahead of LLM-routing rollout — same pattern
//! `routes_genesis::classify_response` uses for archetype
//! classification. Once `argo-provider` exposes a "strongest available
//! model" picker the trait swap is one line.
//!
//! Bearer auth + principal-header validation run in `dispatch_post`;
//! this module assumes both have already passed. Tenant resolution
//! mirrors `routes_skill_ir`: `X-Argo-Tenant-Id` with a workspace-default
//! fallback. The audit row goes out via [`AuditSink`] under
//! `AuditKind::HttpRequest` with `event_type = "skill.ghostwriter.drafted"`.

use super::body_from_request;
use super::routes_auth_principal::principal_id_for_audit;
use super::store_unavailable;
use argo_audit::{AuditKind, AuditSink};
use argo_skill::ir::{
    canonical_hash_hex, ensure_schema, generate_from_prompt, GhostwriterClient, GhostwriterError,
    OfflineGhostwriterClient, SkillIrStore, SqliteSkillIrStore,
};
use argo_store::SessionStore;
use argo_types::{SkillId, TenantId};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

const HEADER_TENANT_ID: &str = "x-argo-tenant-id";
const DEFAULT_WORKSPACE_TENANT_FALLBACK: &str = "00000000-0000-0000-0000-000000000000";
const GHOSTWRITER_CALL_TIMEOUT: Duration = Duration::from_secs(45);
const MIN_PROMPT_LEN: usize = 15;
const DRAFT_VERSION: &str = "1.0.0";

/// `POST /api/v1/skills/draft-from-prompt`. Body must include
/// `{"prompt": "<≥15 chars>"}`. Response on success:
/// `{"skillId","tenantId","version","irHash","graph"}`.
pub(super) async fn draft_from_prompt_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let tenant = match resolve_tenant(req) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let prompt = match parse_prompt(req) {
        Ok(p) => p,
        Err(resp) => return resp,
    };
    let llm: OfflineGhostwriterClient = OfflineGhostwriterClient;
    let graph_result =
        tokio::time::timeout(GHOSTWRITER_CALL_TIMEOUT, run_pipeline(&prompt, &llm)).await;
    let graph = match graph_result {
        Ok(Ok(g)) => g,
        Ok(Err(err)) => return ghostwriter_error_response(&err),
        Err(_) => return timeout_response(),
    };
    let hash = match canonical_hash_hex(&graph) {
        Ok(h) => h,
        Err(e) => return server_error(&format!("canonical_hash: {e}")),
    };
    let graph_value = match serde_json::to_value(&graph) {
        Ok(v) => v,
        Err(e) => return server_error(&format!("graph encode: {e}")),
    };
    if let Err(e) = ensure_schema(store).await {
        return server_error(&format!("skill_ir ensure_schema: {e}"));
    }
    let skill_id = SkillId::new();
    let ir_store = SqliteSkillIrStore::new(store);
    if let Err(e) = ir_store.put(tenant, skill_id, DRAFT_VERSION, &graph).await {
        return server_error(&format!("skill_ir put: {e}"));
    }
    emit_audit(store, req, tenant, skill_id, &hash, prompt.len()).await;
    let body = json!({
        "skillId": skill_id.to_string(),
        "tenantId": tenant.to_string(),
        "version": DRAFT_VERSION,
        "irHash": hash,
        "graph": graph_value,
        "source": "ghostwriter",
    });
    ("200 OK", body.to_string())
}

async fn run_pipeline(
    prompt: &str,
    llm: &dyn GhostwriterClient,
) -> Result<argo_skill::ir::SkillGraph, GhostwriterError> {
    generate_from_prompt(prompt, llm).await
}

fn parse_prompt(req: &str) -> Result<String, (&'static str, String)> {
    let body = body_from_request(req);
    let parsed: Value =
        serde_json::from_str(body).map_err(|e| bad_request(&format!("parse body: {e}")))?;
    let prompt = parsed
        .get("prompt")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .ok_or_else(|| bad_request("`prompt` (string) is required"))?;
    if prompt.len() < MIN_PROMPT_LEN {
        return Err(bad_request(&format!(
            "`prompt` must be at least {MIN_PROMPT_LEN} non-whitespace chars"
        )));
    }
    Ok(prompt.to_string())
}

fn resolve_tenant(req: &str) -> Result<TenantId, (&'static str, String)> {
    let raw = header_value(req, HEADER_TENANT_ID)
        .unwrap_or_else(|| DEFAULT_WORKSPACE_TENANT_FALLBACK.to_string());
    Uuid::parse_str(&raw)
        .map(TenantId)
        .map_err(|e| bad_request(&format!("invalid tenant id '{raw}': {e}")))
}

fn header_value(req: &str, name_lower: &str) -> Option<String> {
    let headers = req
        .split_once("\r\n\r\n")
        .map_or(req, |(headers, _body)| headers);
    headers.lines().skip(1).find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.trim()
            .eq_ignore_ascii_case(name_lower)
            .then(|| value.trim().to_string())
    })
}

async fn emit_audit(
    store: &Arc<SessionStore>,
    req: &str,
    tenant: TenantId,
    skill_id: SkillId,
    ir_hash: &str,
    prompt_length: usize,
) {
    let payload = json!({
        "endpoint": "skill.ghostwriter.drafted",
        "tenant_id": tenant.to_string(),
        "skill_id": skill_id.to_string(),
        "version": DRAFT_VERSION,
        "ir_hash": ir_hash,
        "prompt_length": prompt_length,
    });
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    if let Err(e) = sink
        .write(
            None,
            AuditKind::HttpRequest,
            payload,
            principal_id_for_audit(req),
        )
        .await
    {
        tracing::warn!(
            target: "gateway::ghostwriter",
            error = %e,
            "ghostwriter audit write failed",
        );
    }
}

fn ghostwriter_error_response(err: &GhostwriterError) -> (&'static str, String) {
    match err {
        GhostwriterError::EmptyPrompt => bad_request("`prompt` is empty after trimming"),
        GhostwriterError::Decode(msg) => unprocessable("decode", msg),
        GhostwriterError::Validation(msg) => unprocessable("validation", msg),
        GhostwriterError::Provider(msg) => server_error(&format!("ghostwriter provider: {msg}")),
    }
}

fn bad_request(message: &str) -> (&'static str, String) {
    error_envelope("400 Bad Request", "bad_request", message)
}

fn unprocessable(code: &str, message: &str) -> (&'static str, String) {
    error_envelope("422 Unprocessable Entity", code, message)
}

fn server_error(message: &str) -> (&'static str, String) {
    error_envelope("500 Internal Server Error", "internal_error", message)
}

fn timeout_response() -> (&'static str, String) {
    error_envelope(
        "408 Request Timeout",
        "timeout",
        "Ghostwriter is taking longer than usual. Try again.",
    )
}

fn error_envelope(status: &'static str, code: &str, message: &str) -> (&'static str, String) {
    (
        status,
        json!({"error": {"code": code, "message": message}}).to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req_with(body: &str) -> String {
        format!(
            "POST /api/v1/skills/draft-from-prompt HTTP/1.1\r\n\
             Host: x\r\n\
             X-Argo-Tenant-Id: 00000000-0000-0000-0000-000000000001\r\n\
             Content-Type: application/json\r\n\r\n{body}"
        )
    }

    #[test]
    fn parse_prompt_rejects_short_input() {
        let req = req_with(r#"{"prompt":"hi"}"#);
        let err = parse_prompt(&req).unwrap_err();
        assert!(err.0.starts_with("400"));
        assert!(err.1.contains("at least"));
    }

    #[test]
    fn parse_prompt_rejects_missing_field() {
        let req = req_with(r#"{}"#);
        let err = parse_prompt(&req).unwrap_err();
        assert!(err.0.starts_with("400"));
    }

    #[test]
    fn parse_prompt_accepts_well_formed_body() {
        let req = req_with(r#"{"prompt":"Build a skill that does the thing"}"#);
        let prompt = parse_prompt(&req).unwrap();
        assert!(prompt.starts_with("Build a skill"));
    }

    #[test]
    fn resolve_tenant_falls_back_to_workspace_default() {
        let req = "POST /x HTTP/1.1\r\nHost: x\r\n\r\n";
        let t = resolve_tenant(req).unwrap();
        assert_eq!(t.to_string(), DEFAULT_WORKSPACE_TENANT_FALLBACK);
    }

    #[tokio::test]
    async fn missing_store_yields_503() {
        let resp = draft_from_prompt_response("POST /x HTTP/1.1\r\n\r\n", None).await;
        assert_eq!(resp.0, "503 Service Unavailable");
    }
}
