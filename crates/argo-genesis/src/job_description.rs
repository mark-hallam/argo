//! Mission `mission-genesis-job-description-ingest` (#64 in
//! MISSION_BACKLOG.md Section 3 Claim 3).
//!
//! Adds the "drop in a real job description" onramp so operators
//! don't have to hand-author a charter to onboard a worker. The
//! pipeline reads PDF / plain text / URL, synthesises a charter,
//! then runs the existing Genesis pipeline ([`plan_with`]) to
//! produce the same `GenesisPlan` shape the charter-dialogue
//! path already emits.
//!
//! ## Sensible deviation from the brief (2026-05-11)
//!
//! The brief cites `argo-executor-document::pdf::extract_text` as
//! the PDF extractor. That symbol does not exist — the
//! `argo-executor-document` `pdf` module is the markdown → PDF
//! *output* renderer (headless Chrome). To unblock the mission
//! claim ("operator drops in real JD PDF → working teammate"),
//! this brief pulls the `pdf-extract` crate in directly as a
//! narrow `argo-genesis` dep. A follow-up brief should move the
//! extractor into `argo-executor-document` and switch this
//! module over.
//!
//! ## v1 scope (matches the brief)
//!
//! - PDF: read whole file (no streaming), extract text, error
//!   with a hint if extraction returns whitespace-only ("image-
//!   only PDF, OCR not supported").
//! - URL: reqwest GET, 30s timeout, 5MB body cap, decoded as
//!   UTF-8.
//! - Text: identity.
//! - Charter synthesis: identity-fn stub. When
//!   `mission-genesis-llm-classifier` lands and a `ProviderClient`
//!   is wired through, this becomes a single-pass LLM call.

use crate::archetype::Archetype;
use crate::error::GenesisError;
use crate::eval_seed::{generate as generate_eval_seeds, suggested_actions_from_manifest, EvalSeed};
use crate::pipeline::{plan_with, GenesisConfig, GenesisPlan};
use argo_types::{ArgoError, ArgoResult};
use argo_vertical::VerticalManifest;
use std::path::{Path, PathBuf};
use std::time::Duration;

const URL_BODY_CAP_BYTES: usize = 5 * 1024 * 1024;
const URL_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_PERSONA: &str = "worker";

/// What the operator dropped in. PDF and URL are read in-process;
/// text is the identity path.
#[derive(Debug, Clone)]
pub enum JobDescInput {
    /// Path to a local PDF file. Read in full (no streaming) and
    /// passed through `pdf-extract`.
    Pdf(PathBuf),
    /// Raw plain-text body. Identity.
    Text(String),
    /// `http(s)` URL. Fetched with reqwest; 30s timeout, 5MB cap.
    Url(String),
}

/// The result of running a job description through the Genesis
/// pipeline. Identical to a `GenesisPlan` produced by
/// [`crate::plan_with`], plus the eval seed and the synthesised
/// charter (so operators can copy it into a file and feed
/// `argo genesis from-charter` if they want to edit).
#[derive(Debug, Clone)]
pub struct GenesisDraft {
    /// The charter that was passed to the Genesis pipeline. In v1
    /// this is byte-identical to the extracted text; once the LLM
    /// classifier brief lands it becomes a single-pass synthesis.
    pub synthesized_charter: String,
    /// Resolved archetype.
    pub archetype: Archetype,
    /// Full manifest, ready to write to disk.
    pub manifest: VerticalManifest,
    /// 10 deterministic starter eval cases.
    pub eval_seed: Vec<EvalSeed>,
}

/// Ingest a job description and produce a Genesis draft. Async
/// because URL fetch is async; the PDF and text paths complete
/// synchronously but are wrapped in the same signature so callers
/// don't have to branch on input type.
///
/// # Errors
///
/// Returns [`ArgoError::Schema`] for:
/// - empty / whitespace-only extracted text (image-only PDF hint)
/// - URL body exceeds 5MB cap
/// - URL response was not UTF-8 decodable
/// - PDF decode failure
///
/// Returns [`ArgoError::Provider`] for downstream pipeline
/// failures (reqwest transport, malformed PDF bytes, archetype
/// classification, etc).
pub async fn genesis_from_job_description(
    input: JobDescInput,
) -> ArgoResult<GenesisDraft> {
    let raw = match input {
        JobDescInput::Pdf(path) => extract_pdf_text(&path)?,
        JobDescInput::Text(t) => t,
        JobDescInput::Url(url) => fetch_url(&url).await?,
    };
    if raw.trim().is_empty() {
        return Err(image_only_or_empty_error());
    }
    let synthesized_charter = synthesize_charter(&raw);
    let plan = plan_with(
        &synthesized_charter,
        DEFAULT_PERSONA,
        &GenesisConfig::default(),
        None,
    )
    .await
    .map_err(ArgoError::from)?;
    let eval_seed = derive_eval_seed(&plan)?;
    Ok(GenesisDraft {
        synthesized_charter,
        archetype: plan.archetype,
        manifest: plan.manifest,
        eval_seed,
    })
}

fn extract_pdf_text(path: &Path) -> ArgoResult<String> {
    let bytes = std::fs::read(path).map_err(|e| ArgoError::Io(format!(
        "read {}: {e}",
        path.display()
    )))?;
    pdf_extract::extract_text_from_mem(&bytes).map_err(|e| {
        ArgoError::Schema(format!(
            "pdf extraction failed for {}: {e}",
            path.display()
        ))
    })
}

async fn fetch_url(url: &str) -> ArgoResult<String> {
    let client = reqwest::Client::builder()
        .timeout(URL_TIMEOUT)
        .build()
        .map_err(|e| ArgoError::Provider(format!("url client: {e}")))?;
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| ArgoError::Provider(format!("url fetch {url}: {e}")))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(ArgoError::Provider(format!(
            "url fetch {url}: HTTP {}",
            status.as_u16()
        )));
    }
    if let Some(len) = resp.content_length() {
        if len > URL_BODY_CAP_BYTES as u64 {
            return Err(ArgoError::Schema(format!(
                "url body {len} bytes exceeds {URL_BODY_CAP_BYTES}-byte cap"
            )));
        }
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| ArgoError::Provider(format!("url body {url}: {e}")))?;
    if bytes.len() > URL_BODY_CAP_BYTES {
        return Err(ArgoError::Schema(format!(
            "url body {} bytes exceeds {URL_BODY_CAP_BYTES}-byte cap",
            bytes.len()
        )));
    }
    String::from_utf8(bytes.to_vec()).map_err(|e| {
        ArgoError::Schema(format!("url body not utf-8 ({url}): {e}"))
    })
}

/// v1 stub: pass the extracted text through unchanged. Once
/// `mission-genesis-llm-classifier` lands and exposes a wired
/// `ProviderClient`, this becomes a single-pass call asking the
/// model to compress the JD into ≤500 words of charter.
fn synthesize_charter(raw: &str) -> String {
    raw.to_string()
}

fn derive_eval_seed(plan: &GenesisPlan) -> Result<Vec<EvalSeed>, ArgoError> {
    let actions = plan
        .manifest
        .actions
        .as_ref()
        .map(suggested_actions_from_manifest)
        .unwrap_or_default();
    generate_eval_seeds(&plan.charter, &actions)
}

fn image_only_or_empty_error() -> ArgoError {
    ArgoError::from(GenesisError::Validation {
        stage: "job_description.extract",
        detail: "extracted text is empty; if this is a PDF it may be \
                 image-only (OCR not supported in v1; convert to text \
                 first)"
            .into(),
    })
}

#[cfg(test)]
#[path = "job_description_tests.rs"]
mod tests;
