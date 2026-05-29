//! `Transport` impl that spawns the Claude Code CLI as a subprocess.
//!
//! The driver is intentionally minimal — it shells out to `claude --print
//! --output-format json "<prompt>"`, reads stdout, parses the JSON, and
//! maps it back into the platform's `ProviderResponse` shape. Streaming
//! is faked as a single-chunk stream per the brief's v1 scope.
//!
//! Capabilities: zero net-egress; one env-var read (the
//! `binary_path_env` from the manifest's `[subprocess]` block).
use crate::claude_code_cli::cli_response::ClaudeCliJsonResponse;
use crate::claude_code_cli::prompt_renderer::render_prompt;
use crate::manifest::{ProviderManifest, SubprocessSpec, TransportKind};
use crate::transport::{
    ProviderRequest, ProviderResponse, ProviderResponseDelta, StopReason, Transport,
};
use argo_types::{ArgoError, ArgoResult, CapabilitySet};
use async_trait::async_trait;
use futures::stream::{self as fstream, BoxStream, StreamExt};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

/// Cap stderr bytes attached to errors so a runaway child can't blow up
/// log lines.
const STDERR_CAP: usize = 4096;

/// Manifest-driven Claude Code CLI subprocess transport.
///
/// ```
/// # fn doc() -> argo_types::ArgoResult<()> {
/// use argo_provider::{ClaudeCodeCliTransport, ProviderManifest, Transport};
/// let toml = include_str!("../../../../manifests/providers/claude-code-cli.toml");
/// let manifest = ProviderManifest::from_toml_str(toml)?;
/// let transport = ClaudeCodeCliTransport::from_manifest(manifest)?;
/// assert_eq!(transport.id(), "claude-code-cli");
/// # Ok(()) }
/// ```
pub struct ClaudeCodeCliTransport {
    manifest: ProviderManifest,
    subprocess: SubprocessSpec,
    capabilities: CapabilitySet,
}

impl std::fmt::Debug for ClaudeCodeCliTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClaudeCodeCliTransport")
            .field("provider_id", &self.manifest.provider.id)
            .field("binary_path_env", &self.subprocess.binary_path_env)
            .field("binary_path_fallback", &self.subprocess.binary_path_fallback)
            .field("capabilities", &self.capabilities)
            .finish()
    }
}

impl ClaudeCodeCliTransport {
    /// Build from a parsed manifest. Errors when
    /// `provider.transport_kind != "subprocess"` or the `[subprocess]`
    /// block is absent — both are configuration bugs the caller should
    /// surface up front rather than at first-call time.
    pub fn from_manifest(manifest: ProviderManifest) -> ArgoResult<Self> {
        if manifest.provider.transport_kind != TransportKind::Subprocess {
            return Err(ArgoError::Provider(format!(
                "ClaudeCodeCliTransport requires provider.transport_kind = \"subprocess\"; got {:?}",
                manifest.provider.transport_kind
            )));
        }
        let subprocess = manifest.subprocess.clone().ok_or_else(|| {
            ArgoError::Provider(
                "ClaudeCodeCliTransport requires a [subprocess] manifest block".into(),
            )
        })?;
        let capabilities = manifest.capabilities.to_set(&subprocess.binary_path_env);
        Ok(Self {
            manifest,
            subprocess,
            capabilities,
        })
    }

    /// Borrowed manifest.
    #[must_use]
    pub fn manifest(&self) -> &ProviderManifest {
        &self.manifest
    }

    /// Resolve the binary path — `binary_path_env` first, then
    /// `binary_path_fallback`. Returns the string the driver hands to
    /// `tokio::process::Command::new()`.
    fn resolve_binary(&self) -> String {
        std::env::var(&self.subprocess.binary_path_env)
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| self.subprocess.binary_path_fallback.clone())
    }

    /// Build + spawn the subprocess.
    ///
    /// When `model` is `Some`, `--model <id>` is appended after
    /// `default_args` and before the prompt so the requested tier
    /// (e.g. `claude-sonnet-4-6` for the floor, `claude-opus-4-8` for
    /// complex turns) is pinned. When `None`, no `--model` arg is
    /// passed and the CLI keeps its auto-route behaviour — preserving
    /// backward compatibility for callers that don't pin a model.
    ///
    /// The platform model ids the tier router emits
    /// (`claude-sonnet-4-6`, `claude-opus-4-8`) are exactly the dateless
    /// aliases the `claude` CLI accepts for `--model`, so the value
    /// passes through unchanged.
    fn spawn_child(&self, prompt: &str, model: Option<&str>) -> ArgoResult<tokio::process::Child> {
        let binary = self.resolve_binary();
        let mut cmd = Command::new(&binary);
        cmd.args(&self.subprocess.default_args);
        if let Some(model_id) = model.map(str::trim).filter(|m| !m.is_empty()) {
            cmd.arg("--model").arg(model_id);
        }
        cmd.arg(prompt)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        cmd.spawn().map_err(|e| {
            ArgoError::Provider(format!(
                "claude-code-cli: spawn {binary} failed: {e} \
                 (set ${} to override, or ensure `{}` is on PATH)",
                self.subprocess.binary_path_env, self.subprocess.binary_path_fallback,
            ))
        })
    }

    /// Spawn the CLI with the rendered prompt + parse stdout JSON.
    async fn invoke_cli(
        &self,
        prompt: String,
        model: Option<&str>,
    ) -> ArgoResult<ClaudeCliJsonResponse> {
        let child = self.spawn_child(&prompt, model)?;

        let timeout = Duration::from_millis(self.subprocess.timeout_ms);
        let wait_fut = child.wait_with_output();
        let output = match tokio::time::timeout(timeout, wait_fut).await {
            Ok(Ok(o)) => o,
            Ok(Err(e)) => {
                return Err(ArgoError::Provider(format!(
                    "claude-code-cli: wait failed: {e}"
                )))
            }
            Err(_) => {
                // The wait future owned the child + dropped it on
                // timeout-elapsed; kill_on_drop = true means tokio sent
                // SIGKILL already. Report the deadline cleanly.
                return Err(ArgoError::Provider(format!(
                    "claude-code-cli: subprocess exceeded timeout_ms = {}",
                    self.subprocess.timeout_ms
                )));
            }
        };

        let exit_code = output.status.code();
        let stderr = truncate(String::from_utf8_lossy(&output.stderr).to_string());

        // Claude Code emits a structured JSON envelope on both success
        // AND failure paths — `{ "is_error": true, "result": "<reason>" }`
        // on auth / network / rate-limit errors, exiting non-zero at the
        // same time. Try to parse stdout regardless of exit code so we
        // surface the upstream reason instead of an empty-stderr exit-1.
        let parsed_result: Result<ClaudeCliJsonResponse, serde_json::Error> =
            serde_json::from_slice(&output.stdout);

        if !output.status.success() {
            // Preferred: structured JSON envelope with `is_error=true`.
            if let Ok(parsed) = &parsed_result {
                if parsed.is_error {
                    let body = parsed
                        .result
                        .clone()
                        .unwrap_or_else(|| "(no body)".into());
                    return Err(ArgoError::Provider(format!(
                        "claude-code-cli: is_error=true — {body}"
                    )));
                }
            }
            // Fallback: exit-non-zero with no JSON envelope.
            return Err(ArgoError::Provider(format!(
                "claude-code-cli: exit {} — stderr: {stderr}",
                exit_code
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "signal".into()),
            )));
        }

        let parsed = parsed_result.map_err(|e| {
            let stdout_preview = truncate(String::from_utf8_lossy(&output.stdout).to_string());
            ArgoError::Provider(format!(
                "claude-code-cli: stdout JSON parse: {e} — preview: {stdout_preview}"
            ))
        })?;

        if parsed.is_error {
            let body = parsed.result.unwrap_or_else(|| "(no body)".into());
            return Err(ArgoError::Provider(format!(
                "claude-code-cli: is_error=true — {body}"
            )));
        }
        Ok(parsed)
    }
}

#[async_trait]
impl Transport for ClaudeCodeCliTransport {
    fn id(&self) -> &str {
        &self.manifest.provider.id
    }

    fn capabilities(&self) -> &CapabilitySet {
        &self.capabilities
    }

    async fn complete(&self, req: &ProviderRequest) -> ArgoResult<ProviderResponse> {
        let prompt = render_prompt(req.system.as_deref(), &req.messages);
        // Read stdin separately so we don't accidentally consume from the
        // parent process — `Stdio::null()` in invoke_cli handles this.
        let pinned = request_model(req);
        let parsed = self.invoke_cli(prompt, pinned).await?;
        Ok(parsed_to_response(parsed, &self.manifest.provider.id, pinned))
    }

    async fn complete_stream(
        &self,
        req: &ProviderRequest,
    ) -> ArgoResult<BoxStream<'static, ArgoResult<ProviderResponseDelta>>> {
        // v1 of the CLI driver is one-shot — `claude --print` does not
        // stream. We invoke synchronously + emit the full text as a single
        // delta, then a `FinishReason`. v1.1 will switch to
        // `--output-format stream-json` for real streaming.
        let prompt = render_prompt(req.system.as_deref(), &req.messages);
        let pinned = request_model(req);
        let parsed = self.invoke_cli(prompt, pinned).await?;
        let response = parsed_to_response(parsed, &self.manifest.provider.id, pinned);
        let mut deltas: Vec<ArgoResult<ProviderResponseDelta>> = Vec::new();
        if let Some(text) = response.text.as_ref() {
            if !text.is_empty() {
                deltas.push(Ok(ProviderResponseDelta::TextChunk(text.clone())));
            }
        }
        if let (Some(t_in), Some(t_out)) = (response.tokens_in, response.tokens_out) {
            deltas.push(Ok(ProviderResponseDelta::TokenUsage {
                tokens_in: t_in,
                tokens_out: t_out,
            }));
        }
        deltas.push(Ok(ProviderResponseDelta::FinishReason(response.stop_reason)));
        Ok(fstream::iter(deltas).boxed())
    }
}

/// Extract the model id to pin for this request, or `None` to let the
/// CLI auto-route. An empty `req.model` (the "unpinned" sentinel some
/// callers use) maps to `None` so the CLI's auto-route behaviour is
/// preserved for backward compatibility.
fn request_model(req: &ProviderRequest) -> Option<&str> {
    let trimmed = req.model.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

/// Map a parsed CLI JSON response into the platform's `ProviderResponse`.
/// Pure function — tested in `tests.rs` without spawning a subprocess.
///
/// `pinned_model` is the `--model` id the request pinned, if any. When
/// `Some`, it is authoritative for the reported `provider_meta.model`.
/// This matters because the `claude` CLI fires an internal title /
/// summary side-call (often on Haiku) whose token usage shows up in
/// `modelUsage` — for a short answer that side-call can out-token the
/// main completion, so `dominant_model()` (max output tokens) would
/// mislabel the answering model as Haiku. Pinning the request model as
/// the reported model keeps the audit row honest (the model that
/// answered is the one we asked for) and ensures Haiku never appears as
/// the answering model in the dev routing path. When `None` (unpinned /
/// auto-route), we fall back to `dominant_model()` for backward compat.
pub(super) fn parsed_to_response(
    parsed: ClaudeCliJsonResponse,
    provider_id: &str,
    pinned_model: Option<&str>,
) -> ProviderResponse {
    let stop_reason = parsed
        .stop_reason
        .as_deref()
        .map(map_stop_reason)
        .unwrap_or(StopReason::EndTurn);
    let (tokens_in, tokens_out) = parsed
        .usage
        .as_ref()
        .map(|u| (u.input_tokens, u.output_tokens))
        .unwrap_or((None, None));
    let model = pinned_model
        .map(str::to_string)
        .or_else(|| parsed.dominant_model())
        .unwrap_or_default();
    let provider_meta = serde_json::json!({
        "provider": provider_id,
        "model": model,
    });
    ProviderResponse {
        text: parsed.result,
        reasoning: None,
        tool_calls: Vec::new(),
        stop_reason,
        tokens_in,
        tokens_out,
        provider_meta,
    }
}

fn map_stop_reason(s: &str) -> StopReason {
    match s {
        "end_turn" => StopReason::EndTurn,
        "tool_use" => StopReason::ToolUse,
        "max_tokens" => StopReason::MaxTokens,
        "content_filter" | "stop_sequence" => StopReason::Other(s.to_string()),
        other => StopReason::Other(other.to_string()),
    }
}

fn truncate(s: String) -> String {
    if s.len() <= STDERR_CAP {
        return s;
    }
    let mut end = STDERR_CAP;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    let dropped = s.len() - end;
    format!("{}... [truncated {dropped} bytes]", &s[..end])
}
