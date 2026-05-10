//! Three thin HTTP clients: OpenAI DALL-E 3, Stability AI, Replicate.

use crate::auth::{Backend, BackendConfig};
use crate::b64;

/// Per-backend default base URL, overridable via the `_BASE_URL` test seam.
pub const OPENAI_BASE_URL: &str = "https://api.openai.com";
/// Stability AI default base URL.
pub const STABILITY_BASE_URL: &str = "https://api.stability.ai";
/// Replicate default base URL.
pub const REPLICATE_BASE_URL: &str = "https://api.replicate.com";

const OPENAI_BASE_URL_ENV: &str = "IMAGE_GEN_OPENAI_BASE_URL";
const STABILITY_BASE_URL_ENV: &str = "IMAGE_GEN_STABILITY_BASE_URL";
const REPLICATE_BASE_URL_ENV: &str = "IMAGE_GEN_REPLICATE_BASE_URL";

const REPLICATE_MAX_POLLS: u32 = 30;
const REPLICATE_POLL_DELAY: std::time::Duration = std::time::Duration::from_millis(500);

/// Output of a successful generation: PNG bytes + which backend served it.
#[derive(Debug, Clone)]
pub struct GeneratedImage {
    /// Backend that produced the bytes.
    pub backend: Backend,
    /// PNG bytes.
    pub bytes: Vec<u8>,
}

/// Image-generation client. One instance, three backends.
#[derive(Clone)]
pub struct ImageGenClient {
    http: reqwest::Client,
    cfg: BackendConfig,
    openai_base: String,
    stability_base: String,
    replicate_base: String,
}

impl ImageGenClient {
    /// Build from arg + env precedence (see [`BackendConfig::resolve`]).
    pub fn from_env(backend_arg: Option<&str>) -> Result<Self, String> {
        let cfg = BackendConfig::resolve(backend_arg)?;
        let http = build_http()?;
        Ok(Self {
            http,
            cfg,
            openai_base: env_base_url(OPENAI_BASE_URL_ENV, OPENAI_BASE_URL),
            stability_base: env_base_url(STABILITY_BASE_URL_ENV, STABILITY_BASE_URL),
            replicate_base: env_base_url(REPLICATE_BASE_URL_ENV, REPLICATE_BASE_URL),
        })
    }

    /// Build with explicit backend, key, and a base URL override applied
    /// to the chosen backend's host. Test-only convenience.
    pub fn with_base(backend: Backend, api_key: String, base_url: String) -> Result<Self, String> {
        if api_key.trim().is_empty() {
            return Err(format!("missing {}", backend_cred_env(backend)));
        }
        let http = build_http()?;
        let mut openai_base = OPENAI_BASE_URL.to_string();
        let mut stability_base = STABILITY_BASE_URL.to_string();
        let mut replicate_base = REPLICATE_BASE_URL.to_string();
        match backend {
            Backend::OpenAi => openai_base = base_url,
            Backend::Stability => stability_base = base_url,
            Backend::Replicate => replicate_base = base_url,
        }
        Ok(Self {
            http,
            cfg: BackendConfig { backend, api_key },
            openai_base,
            stability_base,
            replicate_base,
        })
    }

    /// Selected backend.
    pub fn backend(&self) -> Backend {
        self.cfg.backend
    }

    /// Generate one image. Returns PNG bytes labelled with the backend.
    pub async fn generate(
        &self,
        prompt: &str,
        style: Option<&str>,
        size: Option<&str>,
    ) -> Result<GeneratedImage, String> {
        if prompt.trim().is_empty() {
            return Err("missing string field 'prompt'".into());
        }
        let bytes = match self.cfg.backend {
            Backend::OpenAi => self.generate_openai(prompt, style, size).await?,
            Backend::Stability => self.generate_stability(prompt, style, size).await?,
            Backend::Replicate => self.generate_replicate(prompt, style, size).await?,
        };
        if bytes.is_empty() {
            return Err(format!("{}: empty image bytes", self.cfg.backend.name()));
        }
        Ok(GeneratedImage {
            backend: self.cfg.backend,
            bytes,
        })
    }

    async fn generate_openai(
        &self,
        prompt: &str,
        style: Option<&str>,
        size: Option<&str>,
    ) -> Result<Vec<u8>, String> {
        let mut body = serde_json::json!({
            "model": "dall-e-3",
            "prompt": prompt,
            "n": 1,
            "size": size.unwrap_or("1024x1024"),
            "response_format": "b64_json"
        });
        if let Some(s) = style {
            body["style"] = serde_json::Value::String(s.to_string());
        }
        let url = format!("{}/v1/images/generations", self.openai_base);
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.cfg.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("openai POST images.generations: {e}"))?;
        let value = decode_json("openai", "POST /v1/images/generations", resp).await?;
        let encoded = value
            .pointer("/data/0/b64_json")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("openai: missing data[0].b64_json: {value}"))?;
        b64::decode(encoded).map_err(|e| format!("openai: decode base64: {e}"))
    }

    async fn generate_stability(
        &self,
        prompt: &str,
        style: Option<&str>,
        size: Option<&str>,
    ) -> Result<Vec<u8>, String> {
        let mut form = reqwest::multipart::Form::new()
            .text("prompt", prompt.to_string())
            .text("output_format", "png");
        if let Some(s) = style {
            form = form.text("style_preset", s.to_string());
        }
        if let Some((w, h)) = size.and_then(parse_size) {
            form = form
                .text("width", w.to_string())
                .text("height", h.to_string());
        }
        let url = format!("{}/v2beta/stable-image/generate/core", self.stability_base);
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.cfg.api_key)
            .header("accept", "image/*")
            .multipart(form)
            .send()
            .await
            .map_err(|e| format!("stability POST stable-image/generate/core: {e}"))?;
        let status = resp.status();
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| format!("stability: read bytes: {e}"))?;
        if !status.is_success() {
            let text = String::from_utf8_lossy(&bytes);
            return Err(format!("stability: HTTP {status}: {text}"));
        }
        Ok(bytes.to_vec())
    }

    async fn generate_replicate(
        &self,
        prompt: &str,
        style: Option<&str>,
        size: Option<&str>,
    ) -> Result<Vec<u8>, String> {
        let mut input = serde_json::json!({ "prompt": prompt });
        if let Some(s) = style {
            input["style"] = serde_json::Value::String(s.to_string());
        }
        if let Some((w, h)) = size.and_then(parse_size) {
            input["width"] = serde_json::Value::from(w);
            input["height"] = serde_json::Value::from(h);
        }
        let create_url = format!("{}/v1/predictions", self.replicate_base);
        let create_body = serde_json::json!({
            "version": "stability-ai/sdxl",
            "input": input
        });
        let resp = self
            .http
            .post(&create_url)
            .bearer_auth(&self.cfg.api_key)
            .json(&create_body)
            .send()
            .await
            .map_err(|e| format!("replicate POST predictions: {e}"))?;
        let mut value = decode_json("replicate", "POST /v1/predictions", resp).await?;
        for _ in 0..REPLICATE_MAX_POLLS {
            let status = value
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            match status.as_str() {
                "succeeded" => break,
                "failed" | "canceled" => {
                    return Err(format!("replicate: prediction {status}: {value}"));
                }
                _ => {}
            }
            let get_url = value
                .get("urls")
                .and_then(|v| v.get("get"))
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .ok_or_else(|| format!("replicate: missing urls.get: {value}"))?;
            tokio::time::sleep(REPLICATE_POLL_DELAY).await;
            let resp = self
                .http
                .get(&get_url)
                .bearer_auth(&self.cfg.api_key)
                .send()
                .await
                .map_err(|e| format!("replicate GET prediction: {e}"))?;
            value = decode_json("replicate", "GET prediction", resp).await?;
        }
        let output_url = pick_replicate_output(&value)?;
        download_bytes(&self.http, "replicate", &output_url).await
    }
}

/// Extract the first usable output URL from a Replicate prediction.
pub fn pick_replicate_output(value: &serde_json::Value) -> Result<String, String> {
    let output = value
        .get("output")
        .ok_or_else(|| format!("replicate: missing output: {value}"))?;
    if let Some(url) = output.as_str() {
        return Ok(url.to_string());
    }
    if let Some(items) = output.as_array() {
        if let Some(url) = items.first().and_then(|v| v.as_str()) {
            return Ok(url.to_string());
        }
    }
    Err(format!("replicate: unexpected output shape: {value}"))
}

/// Parse a `<width>x<height>` size hint into `(w, h)`.
pub fn parse_size(raw: &str) -> Option<(u32, u32)> {
    let (w, h) = raw.split_once('x').or_else(|| raw.split_once('X'))?;
    let w: u32 = w.trim().parse().ok()?;
    let h: u32 = h.trim().parse().ok()?;
    if w == 0 || h == 0 {
        return None;
    }
    Some((w, h))
}

fn build_http() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(concat!(
            "argo-mcp-connector-image-gen/",
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .map_err(|e| format!("image-gen http client: {e}"))
}

async fn decode_json(
    backend: &str,
    op: &str,
    resp: reqwest::Response,
) -> Result<serde_json::Value, String> {
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("{backend} {op}: read response: {e}"))?;
    let parsed: serde_json::Value = if text.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&text)
            .map_err(|e| format!("{backend} {op}: decode response: {e}: {text}"))?
    };
    if status.is_success() {
        Ok(parsed)
    } else {
        Err(format!("{backend} {op}: HTTP {status}: {parsed}"))
    }
}

async fn download_bytes(
    http: &reqwest::Client,
    backend: &str,
    url: &str,
) -> Result<Vec<u8>, String> {
    let resp = http
        .get(url)
        .send()
        .await
        .map_err(|e| format!("{backend} GET {url}: {e}"))?;
    let status = resp.status();
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("{backend} GET {url}: read bytes: {e}"))?;
    if !status.is_success() {
        let text = String::from_utf8_lossy(&bytes);
        return Err(format!("{backend} GET {url}: HTTP {status}: {text}"));
    }
    Ok(bytes.to_vec())
}

fn env_base_url(env: &str, default: &str) -> String {
    std::env::var(env)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn backend_cred_env(backend: Backend) -> &'static str {
    match backend {
        Backend::OpenAi => crate::auth::OPENAI_API_KEY_ENV,
        Backend::Stability => crate::auth::STABILITY_API_KEY_ENV,
        Backend::Replicate => crate::auth::REPLICATE_API_TOKEN_ENV,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_size_handles_x_separator() {
        assert_eq!(parse_size("512x768"), Some((512, 768)));
        assert_eq!(parse_size("1024X1024"), Some((1024, 1024)));
        assert_eq!(parse_size("nope"), None);
        assert_eq!(parse_size("0x10"), None);
    }

    #[test]
    fn pick_replicate_output_handles_string_and_array() {
        let v = serde_json::json!({"output": "https://cdn.replicate.example/p.png"});
        assert_eq!(
            pick_replicate_output(&v).unwrap(),
            "https://cdn.replicate.example/p.png"
        );
        let v = serde_json::json!({"output": ["https://cdn.replicate.example/q.png"]});
        assert_eq!(
            pick_replicate_output(&v).unwrap(),
            "https://cdn.replicate.example/q.png"
        );
        let err = pick_replicate_output(&serde_json::json!({})).unwrap_err();
        assert!(err.contains("missing output"));
    }

    #[test]
    fn missing_keys_surface_per_backend_env_name() {
        for (backend, env) in [
            (Backend::OpenAi, "OPENAI_API_KEY"),
            (Backend::Stability, "STABILITY_API_KEY"),
            (Backend::Replicate, "REPLICATE_API_TOKEN"),
        ] {
            let err = match ImageGenClient::with_base(
                backend,
                String::new(),
                "http://localhost".into(),
            ) {
                Ok(_) => panic!("expected missing-key error for {env}"),
                Err(e) => e,
            };
            assert!(err.contains(env), "{err}");
        }
    }
}
