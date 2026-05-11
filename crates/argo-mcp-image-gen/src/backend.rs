//! Backend trait + two implementations: [`OpenAiCompatibleBackend`]
//! (DALL-E 3 / SD-class endpoints behind the openai-compat REST shape)
//! and [`StubBackend`] for tests.
//!
//! Pricing table (cents/image, see [`estimate_cost_cents`]):
//! `dall-e-3`=4, `dall-e-3-hd`=8, `dall-e-2`=2, `stable-diffusion-xl`=2,
//! `gpt-image-1-hd`=25 (above the `--require-confirm` 20¢ gate),
//! anything else=4. Hardcoded for v1; values round UP so the
//! `--require-confirm` gate stays conservative.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// One generation/edit response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImageResult {
    /// Where to fetch the image, or a `data:` URL.
    pub url: String,
    /// Inline base64 PNG when the backend returned bytes.
    pub b64_data: Option<String>,
    /// Model that served the request.
    pub model: String,
    /// First 16 hex chars of `sha256(prompt)`.
    pub prompt_hash: String,
}

impl ImageResult {
    /// Compute the 16-char prompt hash used in [`ImageResult::prompt_hash`].
    #[must_use]
    pub fn hash_prompt(prompt: &str) -> String {
        let mut h = Sha256::new();
        h.update(prompt.as_bytes());
        h.finalize().iter().take(8).map(|b| format!("{b:02x}")).collect()
    }
}

/// Pricing in US-cents per image. See module rustdoc for the table.
#[must_use]
pub fn estimate_cost_cents(model: &str) -> u32 {
    match model {
        "dall-e-3" => 4,
        "dall-e-3-hd" => 8,
        "dall-e-2" => 2,
        "stable-diffusion-xl" | "sdxl" => 2,
        "gpt-image-1-hd" => 25,
        _ => 4,
    }
}

/// Backend abstraction. `image.generate` / `image.edit` route here.
#[async_trait]
pub trait ImageBackend: Send + Sync {
    /// Human-readable backend id (for `ServerInfo` + audit rows).
    fn name(&self) -> &str;
    /// Generate one image from `prompt` at `size` using `model`.
    async fn generate(&self, prompt: &str, size: &str, model: &str)
        -> Result<ImageResult, String>;
    /// Edit `image_bytes` using `prompt`. Single-image edit; multi-
    /// frame composition is out of scope for v1.
    async fn edit(&self, image_bytes: &[u8], prompt: &str, model: &str)
        -> Result<ImageResult, String>;
}

/// Deterministic test backend.
#[derive(Debug, Default, Clone)]
pub struct StubBackend;

#[async_trait]
impl ImageBackend for StubBackend {
    fn name(&self) -> &str { "stub" }
    async fn generate(&self, prompt: &str, _size: &str, model: &str)
        -> Result<ImageResult, String>
    {
        Ok(stub_result("c3R1Yg==", prompt, model))
    }
    async fn edit(&self, _bytes: &[u8], prompt: &str, model: &str)
        -> Result<ImageResult, String>
    {
        Ok(stub_result("c3R1Yi1lZGl0", prompt, model))
    }
}

fn stub_result(b64: &str, prompt: &str, model: &str) -> ImageResult {
    ImageResult {
        url: format!("data:image/png;base64,{b64}"),
        b64_data: Some(b64.into()),
        model: model.into(),
        prompt_hash: ImageResult::hash_prompt(prompt),
    }
}

/// OpenAI-compatible REST backend (`/images/generations` + `/images/edits`).
#[derive(Debug, Clone)]
pub struct OpenAiCompatibleBackend {
    api_base: String,
    api_key: String,
    http: reqwest::Client,
}

impl OpenAiCompatibleBackend {
    /// Build with explicit base + key. Empty key returns an error.
    pub fn new(api_base: impl Into<String>, api_key: impl Into<String>) -> Result<Self, String> {
        let api_base = api_base.into();
        let api_key = api_key.into();
        if api_key.trim().is_empty() {
            return Err("OPENAI_API_KEY (or equivalent) is empty".into());
        }
        let http = reqwest::Client::builder()
            .user_agent(concat!("argo-mcp-image-gen/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| format!("http client: {e}"))?;
        Ok(Self { api_base, api_key, http })
    }

    fn endpoint(&self, path: &str) -> String {
        format!("{}/{path}", self.api_base.trim_end_matches('/'))
    }

    async fn finish(
        &self,
        url: &str,
        prompt: &str,
        model: &str,
        builder: reqwest::RequestBuilder,
    ) -> Result<ImageResult, String> {
        let resp = builder.send().await.map_err(|e| format!("POST {url}: {e}"))?;
        let status = resp.status();
        let value: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("POST {url}: decode: {e}"))?;
        if !status.is_success() {
            return Err(format!("POST {url}: HTTP {status}: {value}"));
        }
        let b64 = value.pointer("/data/0/b64_json").and_then(|v| v.as_str()).map(str::to_string);
        let url_out = value
            .pointer("/data/0/url")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| "data:image/png;base64,".into());
        Ok(ImageResult {
            url: url_out,
            b64_data: b64,
            model: model.into(),
            prompt_hash: ImageResult::hash_prompt(prompt),
        })
    }
}

#[async_trait]
impl ImageBackend for OpenAiCompatibleBackend {
    fn name(&self) -> &str { "openai-compatible" }
    async fn generate(&self, prompt: &str, size: &str, model: &str)
        -> Result<ImageResult, String>
    {
        let url = self.endpoint("images/generations");
        let body = serde_json::json!({
            "model": model, "prompt": prompt, "n": 1,
            "size": size, "response_format": "b64_json"
        });
        let req = self.http.post(&url).bearer_auth(&self.api_key).json(&body);
        self.finish(&url, prompt, model, req).await
    }
    async fn edit(&self, image_bytes: &[u8], prompt: &str, model: &str)
        -> Result<ImageResult, String>
    {
        let url = self.endpoint("images/edits");
        let part = reqwest::multipart::Part::bytes(image_bytes.to_vec())
            .file_name("input.png")
            .mime_str("image/png")
            .map_err(|e| format!("multipart mime: {e}"))?;
        let form = reqwest::multipart::Form::new()
            .text("model", model.to_string())
            .text("prompt", prompt.to_string())
            .text("response_format", "b64_json")
            .part("image", part);
        let req = self.http.post(&url).bearer_auth(&self.api_key).multipart(form);
        self.finish(&url, prompt, model, req).await
    }
}
