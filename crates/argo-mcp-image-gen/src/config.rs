//! TOML parsing for the `[mcp.image_gen]` section of `argo.toml`.

use serde::{Deserialize, Serialize};

/// Which backend the server should dispatch to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageGenBackendKind {
    /// `OpenAiCompatibleBackend` targeting DALL-E 3.
    Openai,
    /// `OpenAiCompatibleBackend` targeting a Stable-Diffusion-class
    /// openai-compat host.
    StableDiffusion,
    /// `StubBackend` for tests + offline operator demos.
    Stub,
}

/// `[mcp.image_gen]` shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ImageGenConfig {
    /// Backend dispatch.
    pub backend: ImageGenBackendKind,
    /// Base URL for openai-compat endpoints (ignored by `stub`).
    pub api_base: String,
    /// Default model when the caller omits `model`.
    pub model_default: String,
    /// Default size when the caller omits `size`.
    pub size_default: String,
}

impl Default for ImageGenConfig {
    fn default() -> Self {
        Self {
            backend: ImageGenBackendKind::Stub,
            api_base: "https://api.openai.com/v1".into(),
            model_default: "dall-e-3".into(),
            size_default: "1024x1024".into(),
        }
    }
}

/// Wrapper matching `argo.toml`'s `[mcp.image_gen]` nesting.
#[derive(Debug, Default, Deserialize)]
struct McpRoot {
    #[serde(default)]
    mcp: Option<McpSection>,
}

#[derive(Debug, Default, Deserialize)]
struct McpSection {
    #[serde(default)]
    image_gen: Option<ImageGenConfig>,
}

impl ImageGenConfig {
    /// Load `[mcp.image_gen]` from `argo.toml` text. Returns a clear
    /// error if the section is missing — the server refuses to start
    /// without explicit config.
    pub fn from_toml_str(text: &str) -> Result<Self, String> {
        Self::from_toml_str_at(text, "argo.toml")
    }

    /// Same as [`from_toml_str`] but threads the on-disk path into
    /// error messages so the operator can see *which* file failed
    /// when `--config <path>` points at a non-default location.
    pub fn from_toml_str_at(text: &str, source: &str) -> Result<Self, String> {
        let root: McpRoot =
            toml::from_str(text).map_err(|e| format!("parse {source}: {e}"))?;
        root.mcp
            .and_then(|m| m.image_gen)
            .ok_or_else(|| format!("{source} missing [mcp.image_gen]"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_section() {
        let toml = r#"
            [mcp.image_gen]
            backend = "openai"
            api_base = "https://api.example.com/v1"
            model_default = "dall-e-3"
            size_default = "1024x1024"
        "#;
        let c = ImageGenConfig::from_toml_str(toml).unwrap();
        assert!(matches!(c.backend, ImageGenBackendKind::Openai));
        assert_eq!(c.api_base, "https://api.example.com/v1");
    }

    #[test]
    fn missing_section_returns_error() {
        let err = ImageGenConfig::from_toml_str("[core]\nhome = \"~/.argo\"").unwrap_err();
        assert!(err.contains("missing [mcp.image_gen]"));
    }

    #[test]
    fn stable_diffusion_is_snake_case() {
        let toml = r#"
            [mcp.image_gen]
            backend = "stable_diffusion"
            api_base = "http://localhost:7860/v1"
            model_default = "sdxl"
            size_default = "1024x1024"
        "#;
        let c = ImageGenConfig::from_toml_str(toml).unwrap();
        assert!(matches!(c.backend, ImageGenBackendKind::StableDiffusion));
    }

    /// `from_toml_str_at` must include the supplied source path in
    /// every error so operators see the path they passed at `--config`.
    #[test]
    fn from_toml_str_at_includes_source_in_parse_error() {
        let bad = "this is = not [valid";
        let err =
            ImageGenConfig::from_toml_str_at(bad, "/etc/argo/custom.toml").unwrap_err();
        assert!(
            err.contains("/etc/argo/custom.toml"),
            "expected path in parse error — got {err}"
        );
    }

    /// `from_toml_str_at` must include the supplied source path when
    /// the required `[mcp.image_gen]` section is absent.
    #[test]
    fn from_toml_str_at_includes_source_when_section_missing() {
        let err =
            ImageGenConfig::from_toml_str_at("[core]\nhome = \"~/.argo\"", "custom.toml")
                .unwrap_err();
        assert!(
            err.contains("custom.toml"),
            "expected path in missing-section error — got {err}"
        );
        assert!(err.contains("missing [mcp.image_gen]"));
    }
}
