//! Manifest-driven `Transport` for the `openai-compatible` family.
//!
//! Speaks the OpenAI Chat Completions wire (the de-facto common subset:
//! OpenAI proper, OpenRouter, vLLM, Together, Fireworks, etc).
//!
//! Spec: `docs/plan/argo-build-plan.md` §8 / Phase 2 plan.
//!
//! Streaming over SSE is declared in the manifest but the Phase 2 PR ships
//! non-streaming requests only — agent-loop-level heartbeat policy (§6.4)
//! lands with Agent F.
mod wire;

#[cfg(test)]
mod tests;

use crate::extractors::extract_tool_calls;
use crate::manifest::{ProviderFamily, ProviderManifest};
use crate::transport::{ProviderRequest, ProviderResponse, Transport};
use argo_types::{ArgoError, ArgoResult, CapabilitySet};
use async_trait::async_trait;
use reqwest::Client;
use std::time::Duration;
use wire::{assemble_response, build_request_body, OpenAiResponse};

/// Manifest-driven OpenAI-compatible transport.
///
/// ```
/// # fn doc() -> argo_types::ArgoResult<()> {
/// use argo_provider::{OpenAiCompatTransport, ProviderManifest, Transport};
/// let toml = include_str!("../../../../manifests/providers/openai-compatible.toml");
/// let manifest = ProviderManifest::from_toml_str(toml)?;
/// let transport = OpenAiCompatTransport::from_manifest(manifest, "sk-test")?;
/// let _: &str = transport.id();
/// # Ok(()) }
/// ```
pub struct OpenAiCompatTransport {
    manifest: ProviderManifest,
    api_key: String,
    client: Client,
    capabilities: CapabilitySet,
}

impl std::fmt::Debug for OpenAiCompatTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAiCompatTransport")
            .field("provider_id", &self.manifest.provider.id)
            .field("base_url", &self.manifest.provider.base_url)
            .field("api_key", &"<redacted>")
            .field("capabilities", &self.capabilities)
            .finish()
    }
}

impl OpenAiCompatTransport {
    /// Build a transport from a parsed manifest + an already-resolved API
    /// key. The caller is responsible for reading the env var declared in
    /// `manifest.auth.api_key_env` (Capability::EnvRead enforced upstream).
    pub fn from_manifest(manifest: ProviderManifest, api_key: &str) -> ArgoResult<Self> {
        if manifest.provider.family != ProviderFamily::OpenaiCompatible {
            return Err(ArgoError::Provider(format!(
                "OpenAiCompatTransport requires family=openai-compatible; got {:?}",
                manifest.provider.family
            )));
        }
        if api_key.is_empty() {
            return Err(ArgoError::Provider(format!(
                "missing API key for env {}",
                manifest.auth.api_key_env
            )));
        }
        let connect = Duration::from_millis(manifest.transport.connect_timeout_ms);
        let read = Duration::from_millis(manifest.transport.read_timeout_ms);
        let client = Client::builder()
            .connect_timeout(connect)
            .timeout(read)
            .build()
            .map_err(|e| ArgoError::Provider(format!("reqwest client build: {e}")))?;
        let capabilities = manifest.capabilities.to_set(&manifest.auth.api_key_env);
        Ok(Self {
            manifest,
            api_key: api_key.to_string(),
            client,
            capabilities,
        })
    }

    /// Borrowed manifest — useful for callers that need provider-level config
    /// (cost router, model resolution, …) without re-parsing.
    #[must_use]
    pub fn manifest(&self) -> &ProviderManifest {
        &self.manifest
    }

    pub(crate) fn endpoint(&self) -> String {
        format!(
            "{}{}",
            self.manifest.provider.base_url, self.manifest.transport.chat_path
        )
    }

    pub(crate) fn auth_header_value(&self) -> String {
        self.manifest
            .auth
            .header_format
            .replace("{key}", &self.api_key)
    }
}

#[async_trait]
impl Transport for OpenAiCompatTransport {
    fn id(&self) -> &str {
        &self.manifest.provider.id
    }

    fn capabilities(&self) -> &CapabilitySet {
        &self.capabilities
    }

    async fn complete(&self, req: &ProviderRequest) -> ArgoResult<ProviderResponse> {
        let body = build_request_body(&self.manifest, req)?;
        let url = self.endpoint();
        let auth = self.auth_header_value();
        let header_name = self.manifest.auth.header_name.clone();
        let raw = retry_post_json(
            &self.client,
            &url,
            &header_name,
            &auth,
            &body,
            self.manifest.transport.max_retries,
        )
        .await?;
        let parsed: OpenAiResponse = serde_json::from_value(raw.clone())
            .map_err(|e| ArgoError::Provider(format!("openai response parse: {e}")))?;
        let tool_calls = extract_tool_calls(self.manifest.tool_calls.format, &raw)?;
        Ok(assemble_response(parsed, tool_calls, raw))
    }
}

async fn retry_post_json(
    client: &Client,
    url: &str,
    header_name: &str,
    header_value: &str,
    body: &serde_json::Value,
    max_retries: u32,
) -> ArgoResult<serde_json::Value> {
    // `[transport].max_retries` is honoured here. Adaptive exponential
    // backoff is **deliberately not implemented** in this crate: it requires
    // a runtime sleep primitive, and argo-provider's dep allowlist (§8 PR
    // brief) excludes `tokio` to keep the manifest layer runtime-agnostic.
    // The agent loop (§6.4 — Agent F) adds the backoff timer there, where
    // tokio is already on the dep tree.
    let mut attempt: u32 = 0;
    loop {
        let resp = client
            .post(url)
            .header(header_name, header_value)
            .header("content-type", "application/json")
            .json(body)
            .send()
            .await;
        let outcome = classify_attempt(resp).await;
        match outcome {
            AttemptOutcome::Ok(value) => return Ok(value),
            AttemptOutcome::Fatal(err) => return Err(err),
            AttemptOutcome::Retry(err) => {
                if attempt >= max_retries {
                    return Err(err);
                }
                tracing::warn!(?attempt, error = %err, "openai-compat retry");
                attempt = attempt.saturating_add(1);
            }
        }
    }
}

enum AttemptOutcome {
    Ok(serde_json::Value),
    Retry(ArgoError),
    Fatal(ArgoError),
}

async fn classify_attempt(resp: Result<reqwest::Response, reqwest::Error>) -> AttemptOutcome {
    let resp = match resp {
        Ok(r) => r,
        Err(e) => {
            let retryable = e.is_timeout() || e.is_connect();
            let err = ArgoError::Provider(format!("reqwest send: {e}"));
            return if retryable {
                AttemptOutcome::Retry(err)
            } else {
                AttemptOutcome::Fatal(err)
            };
        }
    };
    let status = resp.status();
    if status.is_success() {
        return match resp.json::<serde_json::Value>().await {
            Ok(v) => AttemptOutcome::Ok(v),
            Err(e) => {
                AttemptOutcome::Fatal(ArgoError::Provider(format!("openai response decode: {e}")))
            }
        };
    }
    let body_text = resp.text().await.unwrap_or_default();
    let err = ArgoError::Provider(format!("openai HTTP {status}: {body_text}"));
    if status.as_u16() == 429 || status.is_server_error() {
        AttemptOutcome::Retry(err)
    } else {
        AttemptOutcome::Fatal(err)
    }
}
