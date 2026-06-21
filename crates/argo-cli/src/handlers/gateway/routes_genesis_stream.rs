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

use super::rate_limit_ip::{IpRateLimiter, RateLimitIpCheckResult};
use super::routes_genesis_stream_helpers::{
    count_eval_seeds, persona_for, provider_key_present, resolve_vertical_dir, sanitize_tenant,
};
// `derive_persona` is exercised only by the unit-test module (it reaches the
// helpers via `super::*`); the production path uses `persona_for`. Gate the
// import to the test build so the bin/clippy build doesn't see it as unused.
#[cfg(test)]
use super::routes_genesis_stream_helpers::derive_persona;
use super::routes_genesis_stream_llm::resolve_plan;
use argo_genesis::write_vertical_with_overlap;
use argo_store::SessionStore;
use frames::{
    open_sse_stream, write_error_event, write_event, write_one_shot_error, write_progress,
};
use serde::Deserialize;
use std::net::IpAddr;
use std::path::Path;
use std::sync::{Arc, OnceLock};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

// LOC headroom: keep new SSE frame writers in `routes_genesis_stream_frames.rs`.
#[path = "routes_genesis_stream_frames.rs"]
mod frames;

pub const PROVIDER_KEY_ENV: &str = "ARGO_GENESIS_PROVIDER_KEY";
// Provider-key aliases that also satisfy the cost guardrail; presence in any suffices.
pub(super) const PROVIDER_KEY_ALIASES: &[&str] = &["ANTHROPIC_API_KEY", "OPENAI_API_KEY"];
const MIN_ROLE_LEN: usize = 15;

/// Per-IP bucket capacity for the ANONYMOUS Genesis SSE authoring stream.
/// `POST /api/v1/genesis` has no auth gate (the stream is peeled off ahead
/// of `require_identity`), so without this a stranger on the open origin can
/// trigger unbounded real LLM authoring runs — a wallet/cost-DoS bounded only
/// by the coarse general per-IP limiter. This dedicated limiter caps the
/// expensive path the same way `public_pitch_limiter` caps the public Q&A
/// route (`routes_pitch_ask_public.rs`).
const GENESIS_STREAM_BUCKET_CAPACITY: u32 = 10;
/// Refill: ~1 token every ~6.7s ⇒ ~9 authoring runs/minute sustained, with a
/// burst of [`GENESIS_STREAM_BUCKET_CAPACITY`]. Tighter than the public-pitch
/// route (genesis is a full authoring pipeline, not a single Q&A) yet generous
/// enough for real pre-public onboarding + demo concurrency.
///
/// NB (topology): keyed on the accept-loop `src_ip`. Under the current
/// SPA-server-side-rewrite + non-XFF-trusting prod topology that `src_ip` is a
/// shared edge IP, so this is effectively a FLEET-WIDE genesis ceiling, not
/// per-user. That is the correct interim posture (it bounds the cost-DoS); it
/// becomes per-user automatically once real client-IP visibility lands (see the
/// XFF/Cloudflare-topology follow-up). Sized as a deliberate fleet ceiling
/// accordingly.
const GENESIS_STREAM_BUCKET_REFILL_PER_SEC: f64 = 0.15;

/// Process-global per-IP limiter dedicated to the anonymous Genesis SSE
/// authoring stream. Lazily built on first use. `trust_xff = false`: the caller
/// passes the already-trusted `src_ip` extracted once at the accept loop, so
/// this limiter never re-parses a spoofable `X-Forwarded-For` itself.
pub(super) fn genesis_stream_limiter() -> &'static Arc<IpRateLimiter> {
    static INSTANCE: OnceLock<Arc<IpRateLimiter>> = OnceLock::new();
    INSTANCE.get_or_init(|| {
        Arc::new(IpRateLimiter::new(
            GENESIS_STREAM_BUCKET_CAPACITY,
            GENESIS_STREAM_BUCKET_REFILL_PER_SEC,
            true,
            false,
        ))
    })
}

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
    /// Genesis Phase-2 access-controls + guardrails (this brief). The visual
    /// "Set boundaries" spec — read-only/read-write per data category, a
    /// dollar approval ceiling, and free-text escalation topics — synthesized
    /// into an enforceable truth-boundary policy. Omitted from the wire when the
    /// operator sets no boundaries, so the wire shape is unchanged otherwise.
    #[serde(default)]
    boundaries: Option<argo_genesis::policy_synth::BoundariesSpec>,
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
    src_ip: IpAddr,
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

    // Per-IP rate limit FIRST — a cheap reject before the provider-key gate,
    // body parse, or any LLM authoring work. This endpoint is anonymous (the
    // SSE stream is peeled off ahead of `require_identity`), so without this
    // a stranger on the open origin could trigger unbounded real LLM runs
    // (wallet/cost-DoS). Keyed on the trusted accept-loop `src_ip`, never a
    // spoofable `X-Forwarded-For`. See `genesis_stream_limiter`.
    if let RateLimitIpCheckResult::Deny { retry_after_secs } = genesis_stream_limiter().check(src_ip)
    {
        return write_one_shot_error(
            out,
            "429 Too Many Requests",
            "rate_limited",
            &format!(
                "Too many Genesis authoring requests; retry in {retry_after_secs}s (per-IP cost guardrail)"
            ),
            &cors_headers,
        )
        .await;
    }

    // Env-var gate next — fail closed before opening the stream so
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
    // Cap the operator-supplied boundaries spec server-side before any policy
    // synthesis runs, so an adversarial/huge spec can't amplify allocation
    // (escalation-topic count + per-topic length; see
    // `argo_genesis::policy_synth_bounds`). Fail closed with an explicit 400.
    if let Some(e) = parsed
        .boundaries
        .as_ref()
        .and_then(|s| s.validate_input_bounds().err())
    {
        return write_one_shot_error(
            out,
            "400 Bad Request",
            "bad_request",
            &e.to_string(),
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

    let (mut plan, gen_mode) = match resolve_plan(&role_description, &persona, store).await {
        Ok(pair) => pair,
        // A role-refusal (the model returned prose / declined a destructive
        // role, so the broad-role parser found no JSON object) is a GRACEFUL,
        // recoverable case: surface the user-facing "couldn't set up this role,
        // try rephrasing" message with `recoverable:true` so the SPA lets the
        // operator reword and retry, instead of dead-ending on a raw
        // "no JSON object" technical error. Every other failure stays a
        // non-recoverable technical error. See
        // fix-genesis-broad-role-graceful-on-non-json-refusal.
        Err(e) if e.is_role_refusal() => {
            return write_error_event(out, "drafting_persona", &e.to_string(), true).await;
        }
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
    super::routes_genesis_stream_intake::apply_intake(body, &mut plan);

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
    // Genesis Phase-2 boundaries (this brief). Synthesize the server-truth echo
    // and best-effort persist the policy TOML next to the vertical when the
    // resolved tenant is a real UUID (the signed-in demo path). The echo always
    // reflects the synthesized policy so the SPA renders server truth at hire.
    let boundaries_echo = super::routes_genesis_stream_boundaries::persist_and_echo_boundaries(
        parsed.boundaries.as_ref(),
        &tenant_id,
        &vertical_dir,
    );
    let final_payload = super::routes_genesis_stream_payload::build_final_payload(
        &plan,
        &vertical_dir,
        &tenant_id,
        &loaded.manifest.vertical.name,
        eval_seed_count,
        &gate,
        dry_run_until.as_deref(),
        sop_status.as_ref().map(|(label, len)| (label.as_str(), *len)),
        boundaries_echo.as_ref(),
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

#[cfg(test)]
#[path = "routes_genesis_stream_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "routes_genesis_stream_unit_tests.rs"]
mod unit_tests;
