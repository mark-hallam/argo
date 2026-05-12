//! Cassette format + load/save/sequential-match. A [`Cassette`] is a
//! versioned JSON file holding an ordered list of [`CassetteExchange`]s.
//! The recorder pops in recorded order; method/URL mismatch is a hard error.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const REDACTED_PLACEHOLDER: &str = "<REDACTED>";

/// Headers retained in cassettes. Sensitive entries in this list are
/// recorded with `<REDACTED>` values (kept for shape, not content).
pub const DEFAULT_HEADER_ALLOWLIST: &[&str] = &[
    "content-type", "accept", "content-length", "user-agent",
    "authorization", "x-api-key", "x-auth-token", "api-key",
    "x-api-token", "proxy-authorization",
];

#[derive(Debug, thiserror::Error)]
pub enum CassetteError {
    #[error("io: {0}")]
    Io(String),
    #[error("json: {0}")]
    Json(String),
    #[error("cassette exhausted at index {0}")]
    Exhausted(usize),
    #[error("cassette mismatch at exchange {index}: expected {expected_method} {expected_url}, got {actual_method} {actual_url}")]
    Mismatch {
        index: usize,
        expected_method: String,
        expected_url: String,
        actual_method: String,
        actual_url: String,
    },
}

pub type Result<T> = std::result::Result<T, CassetteError>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CassetteRequest {
    pub method: String,
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub body_sha256: String,
    pub body_json_redacted: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CassetteResponse {
    pub status: u16,
    pub headers: BTreeMap<String, String>,
    pub body_json: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CassetteExchange {
    pub request: CassetteRequest,
    pub response: CassetteResponse,
    pub recorded_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cassette {
    pub version: String,
    pub exchanges: Vec<CassetteExchange>,
    #[serde(skip)]
    pub(crate) cursor: usize,
    #[serde(skip)]
    pub(crate) path: PathBuf,
}

impl Cassette {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            version: "1".into(),
            exchanges: Vec::new(),
            cursor: 0,
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let bytes = std::fs::read(path.as_ref()).map_err(|e| CassetteError::Io(e.to_string()))?;
        let mut c: Self =
            serde_json::from_slice(&bytes).map_err(|e| CassetteError::Json(e.to_string()))?;
        c.path = path.as_ref().to_path_buf();
        c.cursor = 0;
        Ok(c)
    }

    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| CassetteError::Io(e.to_string()))?;
            }
        }
        let s = serde_json::to_string_pretty(self)
            .map_err(|e| CassetteError::Json(e.to_string()))?;
        std::fs::write(&self.path, s).map_err(|e| CassetteError::Io(e.to_string()))?;
        Ok(())
    }

    pub fn append(&mut self, exchange: CassetteExchange) {
        self.exchanges.push(exchange);
    }

    /// Pop the next exchange, requiring method+URL to match the expected request.
    pub fn pop_next(&mut self, expected: &CassetteRequest) -> Result<CassetteResponse> {
        let idx = self.cursor;
        let recorded = self.exchanges.get(idx).ok_or(CassetteError::Exhausted(idx))?;
        if !recorded.request.method.eq_ignore_ascii_case(&expected.method)
            || recorded.request.url != expected.url
        {
            return Err(CassetteError::Mismatch {
                index: idx,
                expected_method: recorded.request.method.clone(),
                expected_url: recorded.request.url.clone(),
                actual_method: expected.method.clone(),
                actual_url: expected.url.clone(),
            });
        }
        let response = recorded.response.clone();
        self.cursor += 1;
        Ok(response)
    }

    pub fn is_exhausted(&self) -> bool {
        self.cursor >= self.exchanges.len()
    }

    pub fn len(&self) -> usize {
        self.exchanges.len()
    }
    pub fn is_empty(&self) -> bool {
        self.exchanges.is_empty()
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Apply the header allowlist + sensitive-value redaction.
pub fn redact_headers(
    headers: &BTreeMap<String, String>,
    allowlist: &[&str],
) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for (k, v) in headers {
        let key_lower = k.to_ascii_lowercase();
        if !allowlist.iter().any(|a| a.eq_ignore_ascii_case(&key_lower)) {
            continue;
        }
        if is_sensitive_header(&key_lower) {
            out.insert(k.clone(), REDACTED_PLACEHOLDER.to_string());
        } else {
            out.insert(k.clone(), v.clone());
        }
    }
    out
}

fn is_sensitive_header(name_lower: &str) -> bool {
    matches!(
        name_lower,
        "authorization"
            | "x-api-key"
            | "x-auth-token"
            | "api-key"
            | "x-api-token"
            | "proxy-authorization"
    )
}

/// Recursively redact body keys matching `api_key|secret|password|token|bearer`.
pub fn redact_body_json(value: serde_json::Value) -> serde_json::Value {
    use serde_json::Value;
    match value {
        Value::Object(map) => {
            let mut out = serde_json::Map::with_capacity(map.len());
            for (k, v) in map {
                if is_sensitive_body_key(&k) {
                    out.insert(k, Value::String(REDACTED_PLACEHOLDER.to_string()));
                } else {
                    out.insert(k, redact_body_json(v));
                }
            }
            Value::Object(out)
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(redact_body_json).collect()),
        other => other,
    }
}

fn is_sensitive_body_key(key: &str) -> bool {
    let lowered = key.to_ascii_lowercase();
    ["api_key", "secret", "password", "token", "bearer"]
        .iter()
        .any(|needle| lowered.contains(needle))
}
