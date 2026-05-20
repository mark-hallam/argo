//! Context-window sizing policy + config loader extracted from
//! [`crate::turn`] so the parent file stays under the 500-LOC
//! ceiling (HARD RULE 6). Consumed by `turn_helpers_run` via
//! `super::*` re-exports on [`crate::turn`].

use argo_provider::ProviderManifest;
use argo_types::ArgoResult;
use std::path::Path;

pub(crate) fn model_context_window(manifest: &ProviderManifest, model: &str) -> usize {
    let resolved = manifest.resolve_model_id(model);
    manifest
        .models
        .iter()
        .find(|m| m.id == resolved)
        .and_then(|m| m.context_window)
        .map_or(0, |n| n as usize)
}

pub(crate) fn load_context_compress_config(
    cwd: &Path,
) -> ArgoResult<argo_config::ContextCompressConfig> {
    let mut config = argo_config::load_from(cwd)?.runtime.context_compress;
    let local = [cwd.join(".skycot/argo.toml"), cwd.join("argo.toml")]
        .into_iter()
        .find(|path| path.exists());
    if let Some(path) = local {
        config = argo_config::ArgoConfig::load(&path)?
            .runtime
            .context_compress;
    }
    Ok(config)
}

pub(crate) struct ContextWindowPolicy {
    pub(crate) window_tokens: usize,
    pub(crate) compression: argo_runtime::context_compress::CompressionConfig,
}

impl ContextWindowPolicy {
    pub(crate) fn disabled() -> Self {
        Self::from_config(argo_config::ContextCompressConfig::default(), 0)
    }

    pub(crate) fn from_config(
        config: argo_config::ContextCompressConfig,
        window_tokens: usize,
    ) -> Self {
        Self {
            window_tokens,
            compression: argo_runtime::context_compress::CompressionConfig {
                enabled: config.enabled,
                trigger_pct: config.trigger_pct,
                keep_last_turns: config.keep_last_turns as usize,
                summary_max_tokens: config.summary_max_tokens as usize,
            },
        }
    }
}
