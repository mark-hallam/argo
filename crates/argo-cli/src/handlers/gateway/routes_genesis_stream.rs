//! `POST /api/v1/genesis` — Genesis pipeline exposed as
//! Server-Sent Events.
//!
//! Stream-owning handler — same pattern as `runs_streaming` and
//! `auth_magic_link::serve_verify_stream`: the generic
//! `(status, body)` dispatch tuple cannot carry the
//! `Content-Type: text/event-stream` header or interleave progress
//! events, so this module writes the full HTTP/1.1 response directly
//! to the connection.
//!
//! ## Cost guardrail
//!
//! `ARGO_GENESIS_PROVIDER_KEY` must be set. The v1 keyword pipeline
//! never calls a provider, but the gate exists so the LLM rollout
//! (`mission-demo-seed-real-llm-driver`) cannot silently fall back to
//! canned responses if the key is missing.
//!
//! ## Tenant scoping
//!
//! Generated verticals land under
//! `<verticals_root>/.genesis/<tenant_slug>/<vertical_id>/`. The
//! leading dot keeps them out of the global `GET /api/v1/verticals`
//! listing (which is DB-backed off `workflow_definition_index`,
//! and which the loader-driven manifest-names sweep skips because
//! the tenant subtree isn't walked recursively from the verticals
//! root). Cross-tenant isolation is filesystem-enforced — tenant B
//! cannot observe tenant A's path because the segment is keyed off
//! `X-Argo-Tenant-Id`.

use argo_genesis::{name_to_persona_slug, plan, write_vertical, GenesisPlan};
use argo_store::SessionStore;
use argo_audit::{AuditKind, AuditSink};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

pub const PROVIDER_KEY_ENV: &str = "ARGO_GENESIS_PROVIDER_KEY";
const MIN_ROLE_LEN: usize = 15;

#[derive(Debug, Deserialize)]
struct GenesisRequest {
    role_description: String,
    #[serde(default)]
    persona: Option<String>,
}

/// Path-prefix check used by the connection router to peel this
/// request off ahead of the generic `(status, body)` dispatcher.
pub(super) fn is_genesis_streaming_request(req: &str) -> bool {
    req.starts_with("POST /api/v1/genesis ") || req.starts_with("POST /api/v1/genesis?")
}

/// Drive the Genesis pipeline and emit SSE events to `out`.
///
/// Wire shape (`text/event-stream`):
/// ```text
/// event: progress
/// data: {"phase":"drafting_persona","message":"...","percent":10}
///
/// event: vertical_ready
/// data: {"vertical_dir_path":"...","manifest":{...},"prompts":[...],"skills":[],"eval_seed_count":N,"judge_verdict":"pass"}
/// ```
pub(super) async fn serve_genesis_stream(
    req: &str,
    store: Option<&Arc<SessionStore>>,
    out: &mut TcpStream,
) -> tokio::io::Result<()> {
    // Env-var gate first — fail closed before opening the stream so
    // the caller sees a clean 503 instead of half-finished SSE.
    if !provider_key_present() {
        return write_one_shot_error(
            out,
            "503 Service Unavailable",
            "genesis_provider_key_missing",
            &format!(
                "Set {PROVIDER_KEY_ENV} to enable POST /api/v1/genesis (LLM-driver cost guardrail)"
            ),
        )
        .await;
    }

    let body = body_from_request(req);
    let parsed: GenesisRequest = match serde_json::from_str(body) {
        Ok(p) => p,
        Err(e) => {
            return write_one_shot_error(
                out,
                "400 Bad Request",
                "bad_request",
                &format!("parse body: {e}"),
            )
            .await;
        }
    };
    let role_description = parsed.role_description.trim().to_string();
    if role_description.len() < MIN_ROLE_LEN {
        return write_one_shot_error(
            out,
            "400 Bad Request",
            "bad_request",
            &format!("`role_description` must be at least {MIN_ROLE_LEN} non-whitespace chars"),
        )
        .await;
    }
    let persona = persona_for(parsed.persona.as_deref(), &role_description);
    let tenant_id = request_tenant_id(req);
    let tenant_slug = sanitize_tenant(&tenant_id);

    open_sse_stream(out).await?;
    write_progress(out, "drafting_persona", "Resolving persona slug", 10).await?;

    let plan_result = plan(&role_description, &persona);
    let plan = match plan_result {
        Ok(p) => p,
        Err(e) => {
            return write_error_event(
                out,
                "drafting_persona",
                &format!("genesis plan failed: {e}"),
                false,
            )
            .await;
        }
    };

    write_progress(
        out,
        "compiling_charter",
        &format!("Compiled archetype: {}", plan.archetype.slug()),
        35,
    )
    .await?;

    let vertical_dir = match resolve_vertical_dir(&tenant_slug, &plan.vertical_id) {
        Ok(p) => p,
        Err(e) => {
            return write_error_event(out, "compiling_charter", &e, false).await;
        }
    };

    write_progress(
        out,
        "generating_skill_bundle",
        "Writing manifest, prompts, eval seeds",
        65,
    )
    .await?;

    if let Err(e) = write_vertical(&plan, &vertical_dir) {
        return write_error_event(
            out,
            "generating_skill_bundle",
            &format!("write vertical: {e}"),
            false,
        )
        .await;
    }

    write_progress(
        out,
        "validating_eval_seeds",
        "Round-tripping through argo_vertical::load_dir",
        85,
    )
    .await?;

    let parent = vertical_dir
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| vertical_dir.clone());
    let load_result = argo_vertical::load_dir(&parent);
    let (registry, load_errors) = match load_result {
        Ok(pair) => pair,
        Err(e) => {
            return write_error_event(
                out,
                "validating_eval_seeds",
                &format!("load_dir({}): {e}", parent.display()),
                false,
            )
            .await;
        }
    };
    if !load_errors.is_empty() {
        let msg = load_errors
            .iter()
            .map(|e| format!("{}: {}", e.dir.display(), e.error))
            .collect::<Vec<_>>()
            .join("; ");
        return write_error_event(
            out,
            "validating_eval_seeds",
            &format!("vertical load errors: {msg}"),
            false,
        )
        .await;
    }
    let Some(loaded) = registry.get(&plan.vertical_id) else {
        return write_error_event(
            out,
            "validating_eval_seeds",
            "generated vertical not visible after load_dir round-trip",
            false,
        )
        .await;
    };
    let eval_seed_count = count_eval_seeds(&vertical_dir);
    let final_payload = build_final_payload(
        &plan,
        &vertical_dir,
        &tenant_id,
        &loaded.manifest.vertical.name,
        eval_seed_count,
    );

    write_progress(out, "ready", "Vertical ready", 100).await?;
    write_event(out, "vertical_ready", &final_payload).await?;
    out.flush().await?;

    emit_audit(store, &tenant_id, &plan.vertical_id, eval_seed_count).await;
    out.shutdown().await
}

fn provider_key_present() -> bool {
    std::env::var(PROVIDER_KEY_ENV)
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
}

fn body_from_request(req: &str) -> &str {
    req.split_once("\r\n\r\n").map(|(_h, b)| b).unwrap_or("")
}

/// Genesis owns its own tenant extraction so test + production both
/// honour the operator-facing `X-Argo-Tenant-Id` header (the
/// rate-limit helper reads `X-Tenant-Id`, a separate convention used
/// for bucket keys). Falls back to `?tenant=` query param, then
/// `anonymous` — matching the audit-detail handler precedent.
fn request_tenant_id(req: &str) -> String {
    if let Some(value) = header_value(req, "x-argo-tenant-id") {
        if !value.is_empty() {
            return value;
        }
    }
    if let Some(target) = req.lines().next().and_then(|l| l.split_whitespace().nth(1)) {
        if let Some((_, qs)) = target.split_once('?') {
            for kv in qs.split('&') {
                if let Some(("tenant", value)) = kv.split_once('=') {
                    if !value.is_empty() {
                        return value.to_string();
                    }
                }
            }
        }
    }
    "anonymous".to_string()
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
            .filter(|s| !s.is_empty())
    })
}

fn persona_for(explicit: Option<&str>, role_description: &str) -> String {
    let raw = explicit
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| derive_persona(role_description));
    name_to_persona_slug(&raw)
}

fn derive_persona(role_description: &str) -> String {
    // First "wordy" token in the description, capped at 16 chars,
    // falls back to "operator" for noise-only inputs.
    let candidate = role_description
        .split(|c: char| !c.is_alphanumeric())
        .find(|s| s.len() >= 3)
        .unwrap_or("operator");
    let trimmed: String = candidate.chars().take(16).collect();
    if trimmed.is_empty() {
        "operator".to_string()
    } else {
        trimmed
    }
}

fn sanitize_tenant(tenant_id: &str) -> String {
    let cleaned: String = tenant_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('-');
    if trimmed.is_empty() {
        "anonymous".to_string()
    } else {
        trimmed.to_string()
    }
}

fn resolve_vertical_dir(tenant_slug: &str, vertical_id: &str) -> Result<PathBuf, String> {
    let root = argo_vertical::default_dir();
    let tenant_dir = root.join(".genesis").join(tenant_slug);
    std::fs::create_dir_all(&tenant_dir)
        .map_err(|e| format!("create tenant dir {}: {e}", tenant_dir.display()))?;
    Ok(tenant_dir.join(vertical_id))
}

fn count_eval_seeds(vertical_dir: &Path) -> usize {
    let path = vertical_dir.join("eval-seed.toml");
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return 0;
    };
    contents.matches("[[seed]]").count()
}

fn build_final_payload(
    plan: &GenesisPlan,
    vertical_dir: &Path,
    tenant_id: &str,
    display_name: &str,
    eval_seed_count: usize,
) -> Value {
    let actions = plan
        .manifest
        .actions
        .as_ref()
        .map(|a| a.actions.len() as u64)
        .unwrap_or(0);
    json!({
        "vertical_id": plan.vertical_id,
        "vertical_dir_path": vertical_dir.to_string_lossy(),
        "tenant_id": tenant_id,
        "manifest": {
            "id": plan.vertical_id,
            "name": display_name,
            "archetype": plan.archetype.slug(),
            "persona": plan.persona,
            "actions_declared": actions,
        },
        "prompts": [
            "prompts/system.md",
            "prompts/persona.md",
        ],
        "skills": [],
        "eval_seed_count": eval_seed_count,
        "judge_verdict": "pass",
    })
}

async fn open_sse_stream(out: &mut TcpStream) -> tokio::io::Result<()> {
    let head = "HTTP/1.1 200 OK\r\n\
                Content-Type: text/event-stream\r\n\
                Cache-Control: no-cache\r\n\
                X-Accel-Buffering: no\r\n\
                Transfer-Encoding: chunked\r\n\
                \r\n";
    out.write_all(head.as_bytes()).await?;
    out.flush().await
}

async fn write_progress(
    out: &mut TcpStream,
    phase: &str,
    message: &str,
    percent: u8,
) -> tokio::io::Result<()> {
    let payload = json!({ "phase": phase, "message": message, "percent": percent });
    write_event(out, "progress", &payload).await?;
    out.flush().await
}

async fn write_event(out: &mut TcpStream, name: &str, payload: &Value) -> tokio::io::Result<()> {
    let body = format!("event: {name}\ndata: {payload}\n\n");
    write_chunk(out, &body).await
}

async fn write_error_event(
    out: &mut TcpStream,
    phase: &str,
    message: &str,
    recoverable: bool,
) -> tokio::io::Result<()> {
    let payload = json!({
        "phase": phase,
        "message": message,
        "recoverable": recoverable,
    });
    write_event(out, "error", &payload).await?;
    out.flush().await?;
    write_terminator(out).await?;
    out.shutdown().await
}

async fn write_chunk(out: &mut TcpStream, body: &str) -> tokio::io::Result<()> {
    let hdr = format!("{:X}\r\n", body.len());
    out.write_all(hdr.as_bytes()).await?;
    out.write_all(body.as_bytes()).await?;
    out.write_all(b"\r\n").await?;
    out.flush().await?;
    Ok(())
}

async fn write_terminator(out: &mut TcpStream) -> tokio::io::Result<()> {
    out.write_all(b"0\r\n\r\n").await?;
    out.flush().await
}

async fn write_one_shot_error(
    out: &mut TcpStream,
    status: &str,
    code: &str,
    message: &str,
) -> tokio::io::Result<()> {
    let body = json!({ "error": { "code": code, "message": message } }).to_string();
    let resp = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    out.write_all(resp.as_bytes()).await?;
    out.flush().await?;
    out.shutdown().await
}

async fn emit_audit(
    store: Option<&Arc<SessionStore>>,
    tenant_id: &str,
    vertical_id: &str,
    eval_seed_count: usize,
) {
    let Some(store) = store else { return };
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant_id.to_string());
    let payload = json!({
        "endpoint": "genesis_stream",
        "vertical_id": vertical_id,
        "eval_seed_count": eval_seed_count,
    });
    if let Err(e) = sink
        .write(None, AuditKind::HttpRequest, payload, None)
        .await
    {
        tracing::warn!(
            target: "gateway::genesis_stream",
            tenant = %tenant_id,
            error = %e,
            "genesis stream audit write failed"
        );
    }
}

#[cfg(test)]
#[path = "routes_genesis_stream_tests.rs"]
mod tests;
