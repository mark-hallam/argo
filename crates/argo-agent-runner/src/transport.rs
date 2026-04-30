//! Transport construction. Resolves a `ProviderManifest` (loaded
//! from `<cwd>/manifests/providers/<id>.toml`) to a live
//! `Box<dyn Transport>`.
use argo_provider::{
    AnthropicTransport, BedrockTransport, OpenAiCompatTransport, OpenAiResponsesTransport,
    ProviderFamily, ProviderManifest, Transport,
};
use argo_types::{ArgoError, ArgoResult};
use std::path::Path;

/// Resolve a `ProviderManifest` to a live `Transport`. Anthropic /
/// OpenAI families read the API key from `manifest.auth.api_key_env`;
/// Bedrock loads AWS credentials via `aws_config::load_from_env()`
/// (honours the standard `AWS_*` env vars + `~/.aws` profile).
pub(crate) async fn build_transport(manifest: ProviderManifest) -> ArgoResult<Box<dyn Transport>> {
    if matches!(manifest.provider.family, ProviderFamily::Bedrock) {
        let cfg = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        return Ok(Box::new(BedrockTransport::from_manifest(manifest, &cfg)?));
    }
    let api_key_env = manifest.auth.api_key_env.clone();
    let api_key = std::env::var(&api_key_env)
        .map_err(|_| ArgoError::Provider(format!("{api_key_env} not set")))?;
    let t: Box<dyn Transport> = match manifest.provider.family {
        ProviderFamily::Anthropic => {
            Box::new(AnthropicTransport::from_manifest(manifest, &api_key)?)
        }
        ProviderFamily::OpenaiCompatible => {
            Box::new(OpenAiCompatTransport::from_manifest(manifest, &api_key)?)
        }
        ProviderFamily::OpenaiResponses => {
            Box::new(OpenAiResponsesTransport::from_manifest(manifest, &api_key)?)
        }
        // Already handled above; reachable only if a new family is
        // added without wiring it through. Fail loud.
        ProviderFamily::Bedrock => unreachable!("bedrock handled above"),
    };
    Ok(t)
}

/// Read `<cwd>/manifests/providers/<provider-id>.toml` and parse.
pub(crate) fn load_manifest(cwd: &Path, provider_id: &str) -> ArgoResult<ProviderManifest> {
    let path = cwd
        .join("manifests")
        .join("providers")
        .join(format!("{provider_id}.toml"));
    let body = std::fs::read_to_string(&path)
        .map_err(|e| ArgoError::Provider(format!("read {}: {e}", path.display())))?;
    ProviderManifest::from_toml_str(&body)
}
