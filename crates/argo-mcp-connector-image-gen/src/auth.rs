//! Backend selection and per-backend credential resolution.

/// `IMAGE_GEN_BACKEND` env override (`openai` | `stability` | `replicate`).
pub const IMAGE_GEN_BACKEND_ENV: &str = "IMAGE_GEN_BACKEND";
/// OpenAI DALL-E credential env var.
pub const OPENAI_API_KEY_ENV: &str = "OPENAI_API_KEY";
/// Stability AI credential env var.
pub const STABILITY_API_KEY_ENV: &str = "STABILITY_API_KEY";
/// Replicate credential env var.
pub const REPLICATE_API_TOKEN_ENV: &str = "REPLICATE_API_TOKEN";

/// Image-generation backend identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// OpenAI DALL-E 3 (default).
    OpenAi,
    /// Stability AI Stable Image Core.
    Stability,
    /// Replicate hosted prediction API.
    Replicate,
}

impl Backend {
    /// Backend wire name used in args, env, and the JSON response.
    pub fn name(self) -> &'static str {
        match self {
            Self::OpenAi => "openai",
            Self::Stability => "stability",
            Self::Replicate => "replicate",
        }
    }

    /// Parse a backend name from an arg or env value.
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "openai" => Ok(Self::OpenAi),
            "stability" => Ok(Self::Stability),
            "replicate" => Ok(Self::Replicate),
            other => Err(format!(
                "unknown image-gen backend '{other}' (expected openai | stability | replicate)"
            )),
        }
    }

    fn cred_env(self) -> &'static str {
        match self {
            Self::OpenAi => OPENAI_API_KEY_ENV,
            Self::Stability => STABILITY_API_KEY_ENV,
            Self::Replicate => REPLICATE_API_TOKEN_ENV,
        }
    }
}

/// Resolved backend + credential for one call.
#[derive(Debug, Clone)]
pub struct BackendConfig {
    /// Selected backend.
    pub backend: Backend,
    /// API key / token for the selected backend.
    pub api_key: String,
}

impl BackendConfig {
    /// Resolve backend from arg + env precedence and look up credential.
    ///
    /// Precedence:
    /// 1. `arg` if it parses to a known backend.
    /// 2. `IMAGE_GEN_BACKEND` env if set.
    /// 3. Default: `openai` if `OPENAI_API_KEY` is set, else first
    ///    backend with a credential, else schema-only error.
    pub fn resolve(arg: Option<&str>) -> Result<Self, String> {
        Self::resolve_with(arg, &|name| argo_config::cred::get(name).ok())
    }

    /// Same as [`Self::resolve`] with an explicit env lookup. Lets tests
    /// avoid mutating process-wide environment.
    pub fn resolve_with(
        arg: Option<&str>,
        getenv: &dyn Fn(&str) -> Option<String>,
    ) -> Result<Self, String> {
        if let Some(raw) = arg.map(str::trim).filter(|s| !s.is_empty()) {
            let backend = Backend::parse(raw)?;
            return Self::with_required(backend, getenv);
        }
        if let Some(raw) = getenv(IMAGE_GEN_BACKEND_ENV) {
            if !raw.trim().is_empty() {
                let backend = Backend::parse(&raw)?;
                return Self::with_required(backend, getenv);
            }
        }
        if let Ok(cfg) = Self::with_required(Backend::OpenAi, getenv) {
            return Ok(cfg);
        }
        for backend in [Backend::Stability, Backend::Replicate] {
            if let Ok(cfg) = Self::with_required(backend, getenv) {
                return Ok(cfg);
            }
        }
        Err(
            "no image-gen backend configured: set OPENAI_API_KEY, STABILITY_API_KEY, or REPLICATE_API_TOKEN"
                .into(),
        )
    }

    fn with_required(
        backend: Backend,
        getenv: &dyn Fn(&str) -> Option<String>,
    ) -> Result<Self, String> {
        let env = backend.cred_env();
        let api_key = getenv(env).ok_or_else(|| format!("missing {env}"))?;
        if api_key.trim().is_empty() {
            return Err(format!("missing {env}"));
        }
        Ok(Self { backend, api_key })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn fixture(map: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let owned: HashMap<String, String> = map
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        move |name: &str| owned.get(name).cloned()
    }

    #[test]
    fn parse_rejects_unknown_backend() {
        let err = Backend::parse("midjourney").unwrap_err();
        assert!(err.contains("unknown image-gen backend"));
    }

    #[test]
    fn arg_overrides_env_when_known() {
        let env = fixture(&[
            (OPENAI_API_KEY_ENV, "sk-abc"),
            (IMAGE_GEN_BACKEND_ENV, "stability"),
            (STABILITY_API_KEY_ENV, "stab-tok"),
        ]);
        let cfg = BackendConfig::resolve_with(Some("openai"), &env).unwrap();
        assert_eq!(cfg.backend, Backend::OpenAi);
        assert_eq!(cfg.api_key, "sk-abc");
    }

    #[test]
    fn env_picks_backend_when_arg_missing() {
        let env = fixture(&[
            (IMAGE_GEN_BACKEND_ENV, "replicate"),
            (REPLICATE_API_TOKEN_ENV, "rep-tok"),
            (OPENAI_API_KEY_ENV, "sk-not-this"),
        ]);
        let cfg = BackendConfig::resolve_with(None, &env).unwrap();
        assert_eq!(cfg.backend, Backend::Replicate);
        assert_eq!(cfg.api_key, "rep-tok");
    }

    #[test]
    fn defaults_to_openai_when_only_openai_creds_present() {
        let env = fixture(&[(OPENAI_API_KEY_ENV, "sk-default")]);
        let cfg = BackendConfig::resolve_with(None, &env).unwrap();
        assert_eq!(cfg.backend, Backend::OpenAi);
    }

    #[test]
    fn falls_back_to_first_backend_with_credentials() {
        let env = fixture(&[(STABILITY_API_KEY_ENV, "stab-only")]);
        let cfg = BackendConfig::resolve_with(None, &env).unwrap();
        assert_eq!(cfg.backend, Backend::Stability);
    }

    #[test]
    fn missing_credentials_emit_actionable_error() {
        let env = fixture(&[]);
        let err = BackendConfig::resolve_with(None, &env).unwrap_err();
        assert!(err.contains("OPENAI_API_KEY"));
        assert!(err.contains("STABILITY_API_KEY"));
        assert!(err.contains("REPLICATE_API_TOKEN"));
    }
}
