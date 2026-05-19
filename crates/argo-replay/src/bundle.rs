//! Session-bundle assembly for `argo replay export`.
//!
//! Pure logic: takes a list of `(path, bytes)` file entries plus
//! a manifest header and emits a self-contained tar (or tar.gz)
//! archive. Sanitization is regex-based and applied to entries
//! whose path is registered as JSON.
//!
//! The crate is decoupled from `argo-store` / `argo-audit`; the CLI
//! handler does the SQL reads and hands the bytes here. See
//! `crates/argo-cli/src/handlers/replay_export.rs` for that side.
//!
//! ```
//! use argo_replay::bundle::{build_bundle, ReplayBundleFile, BundleOptions};
//! let inputs = vec![ReplayBundleFile::json(
//!     "audit/chain.jsonl",
//!     br#"{"kind":"turn_start"}"#.to_vec(),
//! )];
//! let manifest_prefix = "session_id = \"abc\"\n".to_string();
//! let bytes = build_bundle(
//!     inputs,
//!     manifest_prefix,
//!     BundleOptions { sanitize: false, compressed: true, readme: "" },
//! ).unwrap();
//! assert!(bytes.starts_with(&[0x1f, 0x8b])); // gzip magic
//! ```
use argo_types::{ArgoError, ArgoResult};
use flate2::write::GzEncoder;
use flate2::Compression;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::sync::OnceLock;

/// One file entry in the bundle. JSON entries opt into sanitization
/// when [`BundleOptions::sanitize`] is set; opaque (`raw`) entries
/// pass through untouched.
#[derive(Debug, Clone)]
pub struct ReplayBundleFile {
    /// Path inside the tar archive (e.g. `"audit/chain.jsonl"`).
    pub path: String,
    /// File bytes.
    pub bytes: Vec<u8>,
    /// `true` when this is a JSON payload and sanitization should
    /// apply (string values redacted; field names / structure left
    /// intact). `false` for opaque files (README, verify.txt).
    pub is_json: bool,
}

impl ReplayBundleFile {
    /// New JSON entry — eligible for sanitization.
    #[must_use]
    pub fn json(path: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self {
            path: path.into(),
            bytes,
            is_json: true,
        }
    }

    /// New opaque entry — passes through unchanged regardless of
    /// `sanitize` mode.
    #[must_use]
    pub fn raw(path: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self {
            path: path.into(),
            bytes,
            is_json: false,
        }
    }
}

/// Bundle output mode + redaction toggle.
#[derive(Debug, Clone, Copy)]
pub struct BundleOptions<'a> {
    /// Apply secret/PII redaction to JSON entries.
    pub sanitize: bool,
    /// `true` → tar.gz; `false` → plain tar.
    pub compressed: bool,
    /// README content embedded as `README.md`. Empty string skips
    /// the README entry.
    pub readme: &'a str,
}

/// Build a tarball from `files` + a `manifest_prefix` (TOML text
/// the caller already serialized for the session metadata).
///
/// Steps, in order:
///
/// 1. Optional sanitization of JSON entries.
/// 2. Sort entries by `path` (deterministic ordering).
/// 3. SHA-256 over the concatenation of every file's bytes in
///    sorted order — recorded as `sha256 = "..."` and
///    `sanitized = true|false` in the manifest.
/// 4. tar entries: `manifest.toml`, sorted files, optional
///    `README.md`. Symlink following is explicitly disabled.
/// 5. Optional gzip compression.
///
/// # Errors
/// Returns [`ArgoError::Provider`] on tar / gzip serialization
/// failures (these are write-to-`Vec` so failures are exceptional).
pub fn build_bundle(
    files: Vec<ReplayBundleFile>,
    manifest_prefix: String,
    opts: BundleOptions<'_>,
) -> ArgoResult<Vec<u8>> {
    let mut files = files;
    if opts.sanitize {
        for f in &mut files {
            if f.is_json {
                f.bytes = sanitize_json_bytes(&f.bytes);
            }
        }
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let mut hasher = Sha256::new();
    for f in &files {
        hasher.update(&f.bytes);
    }
    let sha = hex::encode(hasher.finalize());
    let mut manifest = manifest_prefix;
    if !manifest.ends_with('\n') {
        manifest.push('\n');
    }
    manifest.push_str(&format!("sha256 = \"{sha}\"\n"));
    manifest.push_str(&format!("sanitized = {}\n", opts.sanitize));
    manifest.push_str(&format!("file_count = {}\n", files.len()));
    let mut tar_bytes = Vec::new();
    {
        let mut builder = tar::Builder::new(&mut tar_bytes);
        builder.follow_symlinks(false);
        append_entry(&mut builder, "manifest.toml", manifest.as_bytes())?;
        for f in &files {
            append_entry(&mut builder, &f.path, &f.bytes)?;
        }
        if !opts.readme.is_empty() {
            append_entry(&mut builder, "README.md", opts.readme.as_bytes())?;
        }
        builder
            .finish()
            .map_err(|e| ArgoError::Provider(format!("finish tar: {e}")))?;
    }
    if !opts.compressed {
        return Ok(tar_bytes);
    }
    let mut gz = GzEncoder::new(Vec::new(), Compression::default());
    gz.write_all(&tar_bytes)
        .map_err(|e| ArgoError::Provider(format!("gzip write: {e}")))?;
    gz.finish()
        .map_err(|e| ArgoError::Provider(format!("gzip finish: {e}")))
}

fn append_entry<W: Write>(
    builder: &mut tar::Builder<W>,
    path: &str,
    bytes: &[u8],
) -> ArgoResult<()> {
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    header.set_mtime(0);
    header.set_entry_type(tar::EntryType::Regular);
    header.set_cksum();
    builder
        .append_data(&mut header, path, bytes)
        .map_err(|e| ArgoError::Provider(format!("append {path}: {e}")))
}

/// Walk a `serde_json::Value` in place and redact every string
/// value matching the secret/PII patterns. Field names + structure
/// are preserved. Non-string scalars are untouched.
///
/// Public so tests in the CLI handler can re-use the redaction
/// without round-tripping bytes through the tarball.
pub fn sanitize_value(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(s) => {
            if let Some(replacement) = redact_str(s) {
                *s = replacement;
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                sanitize_value(v);
            }
        }
        serde_json::Value::Object(map) => {
            for (_, v) in map.iter_mut() {
                sanitize_value(v);
            }
        }
        _ => {}
    }
}

fn sanitize_json_bytes(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    for line in bytes.split_inclusive(|b| *b == b'\n') {
        let (body, trailing_newline) = match line.last() {
            Some(b'\n') => (&line[..line.len() - 1], true),
            _ => (line, false),
        };
        if body.is_empty() {
            if trailing_newline {
                out.push(b'\n');
            }
            continue;
        }
        match serde_json::from_slice::<serde_json::Value>(body) {
            Ok(mut value) => {
                sanitize_value(&mut value);
                match serde_json::to_vec(&value) {
                    Ok(serialized) => out.extend_from_slice(&serialized),
                    Err(_) => out.extend_from_slice(body),
                }
            }
            Err(_) => out.extend_from_slice(body),
        }
        if trailing_newline {
            out.push(b'\n');
        }
    }
    out
}

/// Returns `Some(replacement)` when `s` matches one of the
/// redaction patterns; otherwise `None`.
fn redact_str(s: &str) -> Option<String> {
    let patterns = redaction_patterns();
    for (label, re) in patterns {
        if re.is_match(s) {
            return Some(format!("{label}-redacted-{}", short_hmac(s)));
        }
    }
    None
}

fn short_hmac(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    let digest = h.finalize();
    hex::encode(&digest[..6])
}

fn redaction_patterns() -> &'static [(&'static str, Regex)] {
    static CELL: OnceLock<Vec<(&'static str, Regex)>> = OnceLock::new();
    CELL.get_or_init(|| {
        let mut v: Vec<(&'static str, Regex)> = Vec::new();
        // Provider keys (Anthropic / OpenAI style).
        if let Ok(re) = Regex::new(r"sk-[A-Za-z0-9_-]{16,}") {
            v.push(("sk", re));
        }
        // JWTs (3 base64url-segments separated by dots, leading eyJ).
        if let Ok(re) = Regex::new(r"eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+") {
            v.push(("jwt", re));
        }
        // Email addresses.
        if let Ok(re) = Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}") {
            v.push(("email", re));
        }
        // E.164-ish phone numbers (10–15 digits, optional leading +).
        if let Ok(re) = Regex::new(r"\+?\d[\d\s().-]{8,}\d") {
            v.push(("phone", re));
        }
        v
    })
}

#[cfg(test)]
#[path = "bundle_tests.rs"]
mod tests;
