//! TOML parsing for the `[mcp.image_gen]` section of `argo.toml`.

use serde::{Deserialize, Serialize};

/// Which backend the server should dispatch to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendKind {
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
    pub backend: BackendKind,
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
            backend: BackendKind::Stub,
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
        let root: McpRoot = toml::from_str(text).map_err(|e| format!("parse argo.toml: {e}"))?;
        root.mcp
            .and_then(|m| m.image_gen)
            .ok_or_else(|| "argo.toml missing [mcp.image_gen]".into())
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
        assert!(matches!(c.backend, BackendKind::Openai));
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
        assert!(matches!(c.backend, BackendKind::StableDiffusion));
    }
}
