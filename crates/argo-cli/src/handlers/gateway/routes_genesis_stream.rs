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
//! ## Cost guardrail + LLM generation
//!
//! `ARGO_GENESIS_PROVIDER_KEY` must be set, OR the operator's
//! `ANTHROPIC_API_KEY` / `OPENAI_API_KEY` must be present (the same
//! keys the LLM driver bills against). When the gate passes, the
//! archetype is chosen by a real model reading the role description
//! (see [`super::routes_genesis_stream_llm`]); the deterministic
//! keyword scorer is the automatic fallback on any provider failure,
//! so the demo never hard-breaks and the wire contract is identical
//! either way. The gate exists so the LLM path cannot silently fall
//! back to canned responses with no provider credential present.
//!
//! ## Tenant scoping
//!
//! Generated verticals land under
//! `<verticals_root>/.genesis/<tenant_slug>/<vertical_id>/`. The
//! leading dot keeps them out of the global `GET /api/v1/verticals`
//! listing (DB-backed off `workflow_definition_index`; the
//! loader-driven manifest sweep skips the un-walked tenant subtree).
//!
//! Under DEC-025 #9 (single-tenant-per-cwd for v1; multi-tenant header
//! routing is the DEFERRED enterprise follow-up) the manifest is written
//! under the SAME **cwd-derived workspace tenant the hire reads under**
//! (`SessionStore::ensure_workspace(name, cwd)` — byte-identical to
//! `routes_workers_hire::resolve_tenant_principal`), NOT the request
//! `X-Argo-Tenant-Id` header. The header used to drive the write segment,
//! but the hire-commit endpoint (`POST /api/v1/workers`) ignores the header
//! and resolves the cwd tenant — so a SPA that sent a fresh/unique session
//! header wrote the manifest under the header slug while the hire searched
//! the cwd slug, and the operator's confirmed digest was compared against a
//! stale/absent manifest → `422 genesis_gate_failed`. Resolving both write
//! and read off the same cwd-derived workspace tenant keeps
//! write-segment == read-segment, so a Genesis vertical the SSE writes is
//! always found by the hire in the same segment. The header is retained only
//! as the fallback when no store is wired (test / CLI callers) and as the
//! display `tenant_id` echoed in the payload. Cross-tenant isolation stays
//! filesystem-enforced once multi-tenant routing lands (the segment is still
//! keyed off the resolved tenant slug).
//!
//! ## Non-determinism robustness — unique `vertical_id` per generation
//!
//! A `/genesis` regen for the same role can yield a different `contract_digest`
//! (non-deterministic LLM). The per-role `vertical_id` (`<archetype>-<persona>`)
//! was STABLE, so a regen between review and hire CLOBBERED the reviewed
//! manifest → confirmed digest stopped matching → spurious `422`.
//! `routes_genesis_stream_unique_id` stamps a unique id per generation so a
//! regen writes a NEW dir, leaving the reviewed one hireable. The digest gate
//! is NOT weakened — it covers the capability projection, not the id.

use super::routes_genesis_stream_llm::resolve_plan;
use argo_genesis::{name_to_persona_slug, write_vertical_with_overlap};
use argo_store::SessionStore;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

pub const PROVIDER_KEY_ENV: &str = "ARGO_GENESIS_PROVIDER_KEY";
// Provider-key aliases that also satisfy the cost guardrail; presence in any suffices.
const PROVIDER_KEY_ALIASES: &[&str] = &["ANTHROPIC_API_KEY", "OPENAI_API_KEY"];
const MIN_ROLE_LEN: usize = 15;

#[derive(Debug, Deserialize)]
struct GenesisRequest {
    role_description: String,
    #[serde(default)]
    persona: Option<String>,
    /// Optional single SOP / playbook document the teammate must comprehend
    /// before hire (Genesis P6 — the moat; DEC-025 #5: text role + *optionally*
    /// one SOP doc). Plain-text body OR a base64-encoded PDF (see
    /// [`sop_document_format`]). When present, the teammate is ingested against
    /// it and must pass a doc-grounded comprehension quiz to clear the gate.
    #[serde(default)]
    sop_document: Option<String>,
    /// How to interpret [`Self::sop_document`]: `"text"` (default) or
    /// `"pdf_base64"` (a base64-encoded PDF the gateway extracts via the same
    /// `pdf_extract` path the job-description ingest uses).
    #[serde(default)]
    sop_document_format: Option<String>,
    /// Operator-supplied filename for the SOP (surfaced on the review screen +
    /// audit row). Defaults to a generic label when absent.
    #[serde(default)]
    sop_filename: Option<String>,
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
    // CORS headers for streaming responses — the normal `(status, body)`
    // dispatcher splices these in via `cors::enhance_status`, but the
    // streaming path writes directly to the TCP socket and so has to
    // build them in itself. Without them, the browser fetch from a
    // cross-origin SPA sees a 200/503 with no `Access-Control-Allow-Origin`
    // and rejects the response as `TypeError: Failed to fetch`. See
    // `fix-spa-genesis-fetch-fails-while-curl-succeeds`.
    let cors_headers = super::cors::cors_headers_for_request(req);

    // Env-var gate first — fail closed before opening the stream so
    // the caller sees a clean 503 instead of half-finished SSE.
    if !provider_key_present() {
        let aliases = PROVIDER_KEY_ALIASES.join(" or ");
        return write_one_shot_error(
            out,
            "503 Service Unavailable",
            "genesis_provider_key_missing",
            &format!(
                "Set {PROVIDER_KEY_ENV} (or {aliases}) to enable POST /api/v1/genesis (LLM-driver cost guardrail)"
            ),
            &cors_headers,
        )
        .await;
    }

    let body = req.split_once("\r\n\r\n").map(|(_h, b)| b).unwrap_or("");
    let parsed: GenesisRequest = match serde_json::from_str(body) {
        Ok(p) => p,
        Err(e) => {
            return write_one_shot_error(
                out,
                "400 Bad Request",
                "bad_request",
                &format!("parse body: {e}"),
                &cors_headers,
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
            &cors_headers,
        )
        .await;
    }
    let persona = persona_for(parsed.persona.as_deref(), &role_description);
    // DEC-025 #9 (single-tenant-per-cwd): write the manifest under the SAME
    // cwd-derived workspace tenant the hire-commit endpoint reads under, so the
    // write segment and the hire's read segment can never diverge (the bug:
    // the SSE wrote under the `X-Argo-Tenant-Id` header slug while the hire
    // ignored the header and searched the cwd slug → `422 genesis_gate_failed`
    // when the SPA sent a fresh session header). When no store is wired (test /
    // CLI callers) we fall back to the request header, preserving prior
    // behaviour for those paths.
    let tenant_id = super::routes_genesis_stream_tenant::resolve_write_tenant(store, req).await;
    let tenant_slug = sanitize_tenant(&tenant_id);

    open_sse_stream(out, &cors_headers).await?;
    write_progress(out, "drafting_persona", "Resolving persona slug", 10).await?;

    let (mut plan, gen_mode) = match resolve_plan(&role_description, &persona).await {
        Ok(pair) => pair,
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
    // Unique id BEFORE the dir is resolved + written, so a regeneration writes a
    // new dir (see the module-level non-determinism section).
    super::routes_genesis_stream_unique_id::apply_unique_vertical_suffix(&mut plan);

    write_progress(
        out,
        "compiling_charter",
        &format!(
            "Compiled archetype: {} (generation: {gen_mode})",
            plan.archetype.slug()
        ),
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

    // `allow_overlap = true`: a regeneration for the same role intentionally
    // produces a sibling that heavily overlaps the prior one (same archetype +
    // actions), which the catalog-authoring overlap guard (`conflict.detect`)
    // would otherwise refuse. Regenerating is the expected Genesis flow, so the
    // SSE path opts out of the refusal; the unique id (fresh dir) is what keeps
    // the new sibling from clobbering the reviewed one.
    if let Err(e) = write_vertical_with_overlap(&plan, &vertical_dir, true) {
        return write_error_event(
            out,
            "generating_skill_bundle",
            &format!("write vertical: {e}"),
            false,
        )
        .await;
    }

    // SOP-upload ingestion (Genesis P6 — the moat; DEC-025 #5). If the operator
    // attached ONE SOP doc, ingest it into tenant-scoped knowledge under
    // `<vertical_dir>/knowledge/` + derive a doc-grounded quiz, BEFORE the hire
    // gate runs (the gate reads it from disk). A supplied-but-unusable SOP
    // (empty / image-only / malformed) fails closed with a precise, recoverable
    // error rather than silently dropping the moat. See `routes_genesis_sop`.
    let sop_status = match super::routes_genesis_sop::ingest_request_sop(
        parsed.sop_document.as_deref(),
        parsed.sop_document_format.as_deref(),
        parsed.sop_filename.as_deref(),
        &super::routes_genesis_sop::role_text_for_plan(&plan),
        &vertical_dir,
    ) {
        super::routes_genesis_sop::SopIngestOutcome::Rejected(reason) => {
            let msg = format!("SOP could not be ingested: {reason}");
            return write_error_event(out, "ingesting_sop", &msg, true).await;
        }
        super::routes_genesis_sop::SopIngestOutcome::Ingested { source_label, quiz_len } => {
            let msg = format!("Ingested SOP '{source_label}' → {quiz_len} comprehension questions");
            write_progress(out, "ingesting_sop", &msg, 80).await?;
            Some((source_label, quiz_len))
        }
        super::routes_genesis_sop::SopIngestOutcome::None => None,
    };

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

    // SAFETY GATE (DEC-025 / Genesis P0). Replaces the prior hardcoded
    // `judge_verdict:"pass"`: run the REAL deterministic eval floor + the
    // LLM semantic safety judge, synchronously, before the teammate is
    // reported ready. A teammate that fails its evals OR the safety judge
    // is surfaced as blocked/needs_review and is NOT hireable — the
    // hire-commit endpoint re-runs the identical gate and refuses to
    // activate it.
    write_progress(
        out,
        "running_safety_gate",
        "Running eval floor + safety judge",
        92,
    )
    .await?;
    let gate = super::routes_genesis_gate::run_hire_gate(&plan, Some(&vertical_dir)).await;
    let dry_run_until = super::routes_genesis_stream_payload::read_dry_run_until(&vertical_dir);
    let final_payload = super::routes_genesis_stream_payload::build_final_payload(
        &plan,
        &vertical_dir,
        &tenant_id,
        &loaded.manifest.vertical.name,
        eval_seed_count,
        &gate,
        dry_run_until.as_deref(),
        sop_status.as_ref().map(|(label, len)| (label.as_str(), *len)),
    );

    write_progress(out, "ready", "Vertical ready", 100).await?;
    write_event(out, "vertical_ready", &final_payload).await?;
    out.flush().await?;

    super::routes_genesis_stream_payload::emit_audit(
        store,
        &tenant_id,
        &plan.vertical_id,
        eval_seed_count,
        gen_mode,
        &gate,
    )
    .await;
    out.shutdown().await
}

fn provider_key_present() -> bool {
    let non_empty = |key: &str| {
        std::env::var(key)
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
    };
    non_empty(PROVIDER_KEY_ENV) || PROVIDER_KEY_ALIASES.iter().any(|k| non_empty(k))
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
    // First "wordy" token (>=3 chars), capped at 16; "operator" on noise.
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

/// Canonical tenant→slug map for the `.genesis/<slug>/` tree. Delegates to
/// the shared [`super::genesis_tenant::sanitize_tenant`] so the slug this
/// writer writes under matches the slug the hire gate + runtime fence read
/// under (P5 (b) tenant-match enforcement — no parallel definition).
fn sanitize_tenant(tenant_id: &str) -> String {
    super::genesis_tenant::sanitize_tenant(tenant_id)
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

async fn open_sse_stream(out: &mut TcpStream, cors_headers: &str) -> tokio::io::Result<()> {
    let head = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: text/event-stream\r\n\
         Cache-Control: no-cache\r\n\
         X-Accel-Buffering: no\r\n\
         Transfer-Encoding: chunked\r\n\
         {cors_headers}\r\n"
    );
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
    cors_headers: &str,
) -> tokio::io::Result<()> {
    let body = json!({ "error": { "code": code, "message": message } }).to_string();
    let resp = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{cors_headers}\r\n{body}",
        body.len()
    );
    out.write_all(resp.as_bytes()).await?;
    out.flush().await?;
    out.shutdown().await
}

#[cfg(test)]
#[path = "routes_genesis_stream_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "routes_genesis_stream_unit_tests.rs"]
mod unit_tests;
