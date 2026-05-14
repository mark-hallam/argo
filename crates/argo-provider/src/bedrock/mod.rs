//! Manifest-driven `Transport` for AWS Bedrock (Converse API).
//!
//! Auth: AWS sigv4 via `aws-config` / `aws-sdk-bedrockruntime` (NOT reqwest).
//! Streaming: HTTP/2 eventstream via the SDK's `ConverseStream`.
//!
//! Spec: <https://docs.aws.amazon.com/bedrock/latest/APIReference/API_runtime_Converse.html>.
mod streaming;
mod wire;

#[cfg(test)]
mod tests;

use crate::breaker_observability::ObservedBreakerTransport;
use crate::circuit::{Breaker, BreakerConfig};
use crate::extractors::extract_tool_calls;
use crate::manifest::{ProviderFamily, ProviderManifest, ToolCallFormat};
use crate::transport::{ProviderRequest, ProviderResponse, ProviderResponseDelta, Transport};
use argo_types::{ArgoError, ArgoResult, CapabilitySet, ToolCallId, ToolCallRequest};
use async_trait::async_trait;
use aws_sdk_bedrockruntime::types as bt;
use aws_sdk_bedrockruntime::Client as BedrockClient;
use aws_smithy_types::Document;
use futures::stream::{self as fstream, BoxStream, StreamExt};
use streaming::{step_stream, StreamRunner};

/// Manifest-driven Bedrock transport.
///
/// ```
/// # async fn doc() -> argo_types::ArgoResult<()> {
/// use argo_provider::{BedrockTransport, ProviderManifest};
/// let toml = include_str!("../../../../manifests/providers/bedrock.toml");
/// let manifest = ProviderManifest::from_toml_str(toml)?;
/// let aws_config = aws_config::load_from_env().await;
/// let _t = BedrockTransport::from_manifest(manifest, &aws_config)?;
/// # Ok(()) }
/// ```
pub struct BedrockTransport {
    manifest: ProviderManifest,
    client: BedrockClient,
    capabilities: CapabilitySet,
}

impl std::fmt::Debug for BedrockTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BedrockTransport")
            .field("provider_id", &self.manifest.provider.id)
            .field("base_url", &self.manifest.provider.base_url)
            .field("capabilities", &self.capabilities)
            .finish()
    }
}

impl BedrockTransport {
    /// Build from a parsed manifest + an already-loaded AWS SDK config.
    /// The caller (typically the agent loop bootstrap) is responsible for
    /// the credential chain — we never read AWS env vars from here.
    pub fn from_manifest(
        manifest: ProviderManifest,
        aws_config: &aws_config::SdkConfig,
    ) -> ArgoResult<Self> {
        if manifest.provider.family != ProviderFamily::Bedrock {
            return Err(ArgoError::Provider(format!(
                "BedrockTransport requires family=bedrock; got {:?}",
                manifest.provider.family
            )));
        }
        let client = BedrockClient::new(aws_config);
        let capabilities = manifest.capabilities.to_set(&manifest.auth.api_key_env);
        Ok(Self {
            manifest,
            client,
            capabilities,
        })
    }

    /// Build from a parsed manifest + AWS SDK config, then wrap the
    /// transport with a per-provider [`ObservedBreakerTransport`]
    /// (HTTP-5xx + network-layer failures trip the breaker, every
    /// state transition emits one `tracing::warn!`).
    ///
    /// Each call returns its own [`Breaker`]; never share across
    /// providers.
    pub fn from_manifest_with_breaker(
        manifest: ProviderManifest,
        aws_config: &aws_config::SdkConfig,
        breaker_config: BreakerConfig,
    ) -> ArgoResult<ObservedBreakerTransport<Self>> {
        let inner = Self::from_manifest(manifest, aws_config)?;
        Ok(ObservedBreakerTransport::new(
            inner,
            Breaker::new(breaker_config),
        ))
    }

    /// Borrowed manifest.
    #[must_use]
    pub fn manifest(&self) -> &ProviderManifest {
        &self.manifest
    }
}

#[async_trait]
impl Transport for BedrockTransport {
    fn id(&self) -> &str {
        &self.manifest.provider.id
    }

    fn capabilities(&self) -> &CapabilitySet {
        &self.capabilities
    }

    async fn complete(&self, req: &ProviderRequest) -> ArgoResult<ProviderResponse> {
        let model = self.manifest.resolve_model_id(&req.model);
        let request_body = wire::build_request_body(&self.manifest, req)?;
        let mut builder = self.client.converse().model_id(model);
        builder = apply_messages(builder, &request_body)?;
        let output = builder
            .send()
            .await
            .map_err(|e| ArgoError::Provider(format!("bedrock converse: {e}")))?;
        let (envelope, mut resp) = wire::render_response(output);
        if matches!(
            self.manifest.tool_calls.format,
            ToolCallFormat::BedrockToolUse
        ) {
            let extracted = extract_tool_calls(self.manifest.tool_calls.format, &envelope)?;
            if !extracted.is_empty() {
                resp.tool_calls = extracted;
            }
        }
        Ok(resp)
    }

    async fn complete_stream(
        &self,
        req: &ProviderRequest,
    ) -> ArgoResult<BoxStream<'static, ArgoResult<ProviderResponseDelta>>> {
        let model = self.manifest.resolve_model_id(&req.model).to_string();
        let request_body = wire::build_request_body(&self.manifest, req)?;
        let mut builder = self.client.converse_stream().model_id(model);
        builder = apply_messages_stream(builder, &request_body)?;
        let output = builder
            .send()
            .await
            .map_err(|e| ArgoError::Provider(format!("bedrock converse_stream: {e}")))?;
        let stream = output.stream;
        Ok(fstream::unfold(
            StreamRunner::new(stream, self.manifest.transport.heartbeat_grace_ms),
            step_stream,
        )
        .boxed())
    }
}

fn apply_messages(
    mut builder: aws_sdk_bedrockruntime::operation::converse::builders::ConverseFluentBuilder,
    body: &serde_json::Value,
) -> ArgoResult<aws_sdk_bedrockruntime::operation::converse::builders::ConverseFluentBuilder> {
    if let Some(serde_json::Value::Array(sys)) = body.get("system") {
        for s in sys {
            if let Some(t) = s.get("text").and_then(|v| v.as_str()) {
                builder = builder.system(bt::SystemContentBlock::Text(t.to_string()));
            }
        }
    }
    if let Some(serde_json::Value::Array(messages)) = body.get("messages") {
        for m in messages {
            builder = builder.messages(json_to_bedrock_message(m)?);
        }
    }
    Ok(builder)
}

fn apply_messages_stream(
    mut builder: aws_sdk_bedrockruntime::operation::converse_stream::builders::ConverseStreamFluentBuilder,
    body: &serde_json::Value,
) -> ArgoResult<
    aws_sdk_bedrockruntime::operation::converse_stream::builders::ConverseStreamFluentBuilder,
> {
    if let Some(serde_json::Value::Array(sys)) = body.get("system") {
        for s in sys {
            if let Some(t) = s.get("text").and_then(|v| v.as_str()) {
                builder = builder.system(bt::SystemContentBlock::Text(t.to_string()));
            }
        }
    }
    if let Some(serde_json::Value::Array(messages)) = body.get("messages") {
        for m in messages {
            builder = builder.messages(json_to_bedrock_message(m)?);
        }
    }
    Ok(builder)
}

fn json_to_bedrock_message(m: &serde_json::Value) -> ArgoResult<bt::Message> {
    let role = match m.get("role").and_then(|v| v.as_str()) {
        Some("assistant") => bt::ConversationRole::Assistant,
        _ => bt::ConversationRole::User,
    };
    let content_blocks = m
        .get("content")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(json_to_bedrock_content_block)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    bt::Message::builder()
        .role(role)
        .set_content(Some(content_blocks))
        .build()
        .map_err(|e| ArgoError::Provider(format!("bedrock message build: {e}")))
}

fn json_to_bedrock_content_block(v: &serde_json::Value) -> Option<bt::ContentBlock> {
    if let Some(t) = v.get("text").and_then(|x| x.as_str()) {
        return Some(bt::ContentBlock::Text(t.to_string()));
    }
    if let Some(tu) = v.get("toolUse") {
        let id = tu.get("toolUseId")?.as_str()?.to_string();
        let name = tu.get("name")?.as_str()?.to_string();
        let input = json_to_document(tu.get("input").unwrap_or(&serde_json::Value::Null));
        let block = bt::ToolUseBlock::builder()
            .tool_use_id(id)
            .name(name)
            .input(input)
            .build()
            .ok()?;
        return Some(bt::ContentBlock::ToolUse(block));
    }
    if let Some(tr) = v.get("toolResult") {
        let id = tr.get("toolUseId")?.as_str()?.to_string();
        let body = tr
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|a| a.first())
            .and_then(|c| c.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();
        let block = bt::ToolResultBlock::builder()
            .tool_use_id(id)
            .content(bt::ToolResultContentBlock::Text(body))
            .build()
            .ok()?;
        return Some(bt::ContentBlock::ToolResult(block));
    }
    None
}

fn json_to_document(v: &serde_json::Value) -> Document {
    match v {
        serde_json::Value::Null => Document::Null,
        serde_json::Value::Bool(b) => Document::Bool(*b),
        serde_json::Value::Number(n) => n
            .as_f64()
            .map(|f| Document::Number(aws_smithy_types::Number::Float(f)))
            .unwrap_or(Document::Null),
        serde_json::Value::String(s) => Document::String(s.clone()),
        serde_json::Value::Array(arr) => {
            Document::Array(arr.iter().map(json_to_document).collect())
        }
        serde_json::Value::Object(map) => Document::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), json_to_document(v)))
                .collect(),
        ),
    }
}

/// Manual tool-call construction from the JSON envelope produced by
/// [`wire::render_response`]. Used by the cross-family extractor in
/// [`crate::extractors::bedrock_tool_use`] when the manifest declares
/// `format = "bedrock_tool_use"`.
#[allow(dead_code)]
pub(crate) fn tool_calls_from_envelope(
    body: &serde_json::Value,
) -> ArgoResult<Vec<ToolCallRequest>> {
    let Some(content) = body
        .get("output")
        .and_then(|o| o.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_array())
    else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for block in content {
        let Some(tu) = block.get("toolUse") else {
            continue;
        };
        let name = tu
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ArgoError::Provider("bedrock toolUse missing `name`".into()))?
            .to_string();
        let arguments = tu
            .get("input")
            .cloned()
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
        let provider_call_id = tu
            .get("toolUseId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        out.push(ToolCallRequest {
            id: ToolCallId::new(),
            provider_call_id,
            name,
            arguments,
        });
    }
    Ok(out)
}
