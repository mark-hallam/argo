//! `vision_analyze` — ask a question grounded in an image
//! through the agent's currently-bound provider's native
//! multimodal surface (Hermes parity, brief
//! `tool-vision-analyze`).
//!
//! Capability gate: declared via existing
//! [`argo_types::Capability::ToolDispatch`] with
//! `tool_name_glob = "vision_analyze"` — no new capability
//! variant needed (HARD RULE 5: do not touch
//! `crates/argo-types/src/capability.rs`).
//!
//! Image inputs accepted:
//!
//! | input shape | resolution path |
//! |---|---|
//! | `file://<absolute>` | strip scheme, read bytes via `tokio::fs`, mime from extension |
//! | absolute path | read bytes, mime from extension |
//! | relative path | join workspace root, [`crate::confine_path`] guard, read bytes |
//! | `https://…` | passed through as URL — provider fetches |
//! | `data:<mime>;base64,…` | decoded to bytes + mime |
//!
//! Local file paths go through [`crate::confine_path`] so an
//! agent cannot ask `vision_analyze` to read
//! `/etc/shadow.png` (#677, runtime workspace containment).
//!
//! ## Provider routing
//!
//! Provider routing reuses whichever transport the agent loop
//! has bound to the session — see the `Vision::new` wired form
//! and the [`VisionDispatcher`] trait. The default-constructed
//! form returns a clear "vision provider not wired" error so
//! the existing `builtin_tools(cwd)` call sites keep their
//! behaviour. This mirrors the [`crate::Memory`] pattern.
//!
//! Provider-side image-block shaping (Anthropic
//! `image` / `source.type=base64|url`, OpenAI
//! `image_url`, Bedrock Claude vision body) lives in the
//! provider crates and is wired by the agent runner — out of
//! scope for this brief which only touches `argo-tool` and
//! one vertical manifest.
//!
//! ## Example
//!
//! ```
//! use argo_tool::Vision;
//! // Default form — schema present, dispatch returns a clean
//! // error until the agent runner wires a `VisionDispatcher`.
//! let _v = Vision::default();
//! ```
use crate::path_guard::confine_path;
use crate::tool::{error_text, success_text, Tool, ToolContext};
use argo_types::{ArgoError, ArgoResult, Capability, CapabilitySet, ToolCallResult};
use async_trait::async_trait;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Instant;

/// Maximum decoded image size in bytes — keeps a runaway
/// `data:` URL or oversize attachment from blowing up the
/// agent's context budget. 20 MiB matches the upper end of
/// what Anthropic / OpenAI accept on their vision endpoints.
const MAX_IMAGE_BYTES: usize = 20 * 1024 * 1024;

/// `vision_analyze` tool — schema present always, dispatch
/// errors cleanly until [`Vision::new`] is used.
#[derive(Debug, Default)]
pub struct Vision {
    wired: Option<Arc<dyn VisionDispatcher>>,
}

impl Vision {
    /// Build a `Vision` tool wired to a live dispatcher. The
    /// agent runner constructs this once per turn from the
    /// session's bound provider transport.
    #[must_use]
    pub fn new(dispatcher: Arc<dyn VisionDispatcher>) -> Self {
        Self {
            wired: Some(dispatcher),
        }
    }
}

/// Resolved image payload handed to a [`VisionDispatcher`].
/// Either inline bytes (with mime) or a remote URL the
/// provider should fetch — `https://` URLs pass through
/// rather than being downloaded by the tool, since both
/// Anthropic and OpenAI accept `source.type = url` /
/// `image_url` directly.
#[derive(Debug, Clone)]
pub enum ResolvedImage {
    /// Inline image bytes + mime type (e.g. `"image/png"`).
    Bytes {
        /// Raw image bytes.
        bytes: Vec<u8>,
        /// MIME type.
        mime: String,
    },
    /// Remote `https://` URL the provider will fetch.
    Url(String),
}

/// Provider-bound dispatcher for `vision_analyze`. Implemented
/// by the agent runner's provider-routing layer (out of scope
/// for this brief — the per-provider image-block shaping
/// lives in `argo-provider` and is wired in a follow-up).
#[async_trait]
pub trait VisionDispatcher: Send + Sync + std::fmt::Debug {
    /// Stable provider id (`"anthropic"`, `"openai"`,
    /// `"bedrock"`) — surfaced in the tool's success payload
    /// so the model can cite the source.
    fn provider_id(&self) -> &str;

    /// Send the image + question to the provider and return
    /// the assistant's answer + a token estimate.
    async fn analyze(
        &self,
        image: ResolvedImage,
        question: &str,
    ) -> Result<VisionAnswer, String>;
}

/// Provider's answer to a `vision_analyze` call.
#[derive(Debug, Clone)]
pub struct VisionAnswer {
    /// Assistant text answering the question.
    pub answer: String,
    /// Provider-reported total token usage for the call.
    pub tokens_used: u32,
}

#[derive(Debug, Deserialize)]
struct VisionArgs {
    image: String,
    question: String,
}

#[async_trait]
impl Tool for Vision {
    fn name(&self) -> &str {
        "vision_analyze"
    }

    fn description(&self) -> &str {
        "Ask a question grounded in an image. `image` accepts \
         a file:// path, an absolute filesystem path, an \
         https:// URL, or a data: URL. `question` is the \
         natural-language question to ask about the image. \
         Returns the model's answer routed through the \
         agent's currently-bound provider's native vision \
         surface."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["image", "question"],
            "properties": {
                "image": {
                    "type": "string",
                    "description": "file:// path, absolute path, https:// URL, or data: URL.",
                },
                "question": {
                    "type": "string",
                    "description": "Natural-language question to ask about the image.",
                },
            },
            "additionalProperties": false,
        })
    }

    fn required_capabilities(&self, args: &serde_json::Value) -> ArgoResult<CapabilitySet> {
        let parsed = parse_args(args)?;
        let mut caps = vec![Capability::ToolDispatch {
            tool_name_glob: "vision_analyze".into(),
        }];
        // Path-shaped images need FsRead bound to the resolved
        // path; URL forms (https/data) need none — the provider
        // already holds NetEgress for the model endpoint.
        if let Some(read_glob) = fs_read_glob(&parsed.image) {
            caps.push(Capability::FsRead {
                path_glob: read_glob,
            });
        }
        Ok(CapabilitySet(caps))
    }

    async fn dispatch(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> ArgoResult<ToolCallResult> {
        let started = Instant::now();
        let parsed = parse_args(&args)?;
        let Some(dispatcher) = self.wired.as_ref() else {
            return Ok(error_text(
                ctx.call_id,
                "vision provider not wired — register via \
                 `Vision::new(dispatcher)` (the agent runner \
                 wires this from the session's bound provider) \
                 instead of `builtin_tools(cwd)`'s default."
                    .into(),
                started.elapsed().as_millis() as u64,
            ));
        };
        let image = match resolve_image(&parsed.image, &ctx.workspace_root).await {
            Ok(img) => img,
            Err(msg) => {
                return Ok(error_text(
                    ctx.call_id,
                    format!("vision_analyze: {msg}"),
                    started.elapsed().as_millis() as u64,
                ));
            }
        };
        match dispatcher.analyze(image, &parsed.question).await {
            Ok(answer) => {
                let payload = serde_json::json!({
                    "answer": answer.answer,
                    "provider": dispatcher.provider_id(),
                    "tokens_used": answer.tokens_used,
                });
                Ok(success_text(
                    ctx.call_id,
                    serde_json::to_string(&payload).unwrap_or_default(),
                    started.elapsed().as_millis() as u64,
                ))
            }
            Err(msg) => Ok(error_text(
                ctx.call_id,
                format!("vision_analyze: provider call failed: {msg}"),
                started.elapsed().as_millis() as u64,
            )),
        }
    }
}

fn parse_args(args: &serde_json::Value) -> ArgoResult<VisionArgs> {
    serde_json::from_value(args.clone()).map_err(|e| ArgoError::Tool {
        tool: "vision_analyze".into(),
        msg: format!("invalid arguments: {e}"),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImageKind {
    DataUrl,
    HttpsUrl,
    FileUrl,
    Path,
}

fn image_kind(image: &str) -> ImageKind {
    if image.starts_with("data:") {
        ImageKind::DataUrl
    } else if image.starts_with("https://") || image.starts_with("http://") {
        ImageKind::HttpsUrl
    } else if image.starts_with("file://") {
        ImageKind::FileUrl
    } else {
        ImageKind::Path
    }
}

fn fs_read_glob(image: &str) -> Option<String> {
    match image_kind(image) {
        ImageKind::DataUrl | ImageKind::HttpsUrl => None,
        ImageKind::FileUrl => Some(image.trim_start_matches("file://").to_string()),
        ImageKind::Path => Some(image.to_string()),
    }
}

async fn resolve_image(
    image: &str,
    workspace_root: &std::path::Path,
) -> Result<ResolvedImage, String> {
    match image_kind(image) {
        ImageKind::DataUrl => decode_data_url(image),
        ImageKind::HttpsUrl => Ok(ResolvedImage::Url(image.to_string())),
        ImageKind::FileUrl => {
            let stripped = image.trim_start_matches("file://");
            read_file_bytes(workspace_root, std::path::Path::new(stripped)).await
        }
        ImageKind::Path => {
            read_file_bytes(workspace_root, std::path::Path::new(image)).await
        }
    }
}

async fn read_file_bytes(
    workspace_root: &std::path::Path,
    requested: &std::path::Path,
) -> Result<ResolvedImage, String> {
    let resolved = confine_path(workspace_root, requested)?;
    let bytes = tokio::fs::read(&resolved)
        .await
        .map_err(|e| format!("read '{}': {e}", resolved.display()))?;
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(format!(
            "image at '{}' is {} bytes; exceeds {}-byte ceiling",
            resolved.display(),
            bytes.len(),
            MAX_IMAGE_BYTES
        ));
    }
    let mime = mime_from_extension(&resolved)?;
    Ok(ResolvedImage::Bytes { bytes, mime })
}

/// Restricted to the set of mime types the major vision
/// providers accept; failing fast here yields a better error
/// than a 400 from the model API.
fn mime_from_extension(path: &std::path::Path) -> Result<String, String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .ok_or_else(|| {
            format!(
                "image '{}' has no file extension; cannot infer mime",
                path.display()
            )
        })?;
    match ext.as_str() {
        "png" => Ok("image/png".into()),
        "jpg" | "jpeg" => Ok("image/jpeg".into()),
        "gif" => Ok("image/gif".into()),
        "webp" => Ok("image/webp".into()),
        other => Err(format!(
            "unsupported image extension '.{other}' (allowed: png, jpg, jpeg, gif, webp)"
        )),
    }
}

fn decode_data_url(url: &str) -> Result<ResolvedImage, String> {
    let body = url
        .strip_prefix("data:")
        .ok_or_else(|| "data url missing 'data:' prefix".to_string())?;
    let (header, payload) = body
        .split_once(',')
        .ok_or_else(|| "data url missing ',' separator".to_string())?;
    let (mime, encoding) = header
        .split_once(';')
        .ok_or_else(|| format!("data url header '{header}' missing ';base64'"))?;
    if encoding.trim() != "base64" {
        return Err(format!(
            "data url encoding '{encoding}' not supported (require ';base64')"
        ));
    }
    if mime.is_empty() {
        return Err("data url mime is empty".into());
    }
    let bytes = base64_decode(payload.trim())?;
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(format!(
            "data url payload is {} bytes; exceeds {}-byte ceiling",
            bytes.len(),
            MAX_IMAGE_BYTES
        ));
    }
    Ok(ResolvedImage::Bytes {
        bytes,
        mime: mime.to_string(),
    })
}

/// Minimal base64 decoder — avoids pulling a new dep just
/// for the data-url path. Whitespace ignored; padding (`=`)
/// honoured but optional.
fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    const TABLE: [i8; 256] = build_b64_table();
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits: u8 = 0;
    for b in input.bytes() {
        if b == b'=' {
            break;
        }
        if b.is_ascii_whitespace() {
            continue;
        }
        let v = TABLE[b as usize];
        if v < 0 {
            return Err(format!("invalid base64 char: {b:#x}"));
        }
        buf = (buf << 6) | (v as u32);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((buf >> bits) & 0xff) as u8);
        }
    }
    Ok(out)
}

const fn build_b64_table() -> [i8; 256] {
    let mut t = [-1i8; 256];
    let alpha = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut i = 0;
    while i < alpha.len() {
        t[alpha[i] as usize] = i as i8;
        i += 1;
    }
    t
}

#[cfg(test)]
#[path = "vision_tests.rs"]
mod tests;
