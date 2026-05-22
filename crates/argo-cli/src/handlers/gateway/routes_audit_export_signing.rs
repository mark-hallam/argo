//! Signing, path-safety, and URL helpers for
//! [`super::routes_audit_export`]. Split out of the main file so the
//! handlers fit under the 500-LOC ceiling after
//! `mission-audit-export-tenant-scope-fix` added tenant binding.

use argo_audit::export_bundle::BundleFormat;
use argo_audit::DEFAULT_AUDIT_TENANT_ID;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

const SECRET_ENV: &str = "ARGO_AUDIT_EXPORT_SECRET";
const DEV_FALLBACK_SECRET: &[u8] = b"argo-dev-export-secret-do-not-use-in-prod";
const EXPORT_DIR_ENV: &str = "ARGO_AUDIT_EXPORT_DIR";

/// Build the public download URL for `(bundle_id, format)`, signed
/// for `tenant_id`. The download handler matches the URL's `tenant`
/// query against the caller's request tenant header before reading
/// bytes (`mission-audit-export-tenant-scope-fix`).
#[must_use]
pub(super) fn sign_export_url(
    bundle_id: &str,
    format: BundleFormat,
    expires: i64,
    tenant_id: &str,
) -> String {
    let sig = compute_signature(bundle_id, format, expires, tenant_id);
    format!(
        "/api/v1/audit/export/{bundle_id}?format={}&expires={expires}&tenant={}&sig={sig}",
        format_wire(format),
        url_encode(tenant_id)
    )
}

pub(super) fn format_wire(f: BundleFormat) -> &'static str {
    match f {
        BundleFormat::Json => "json",
        BundleFormat::Csv => "csv",
        BundleFormat::Tar => "tar",
    }
}

pub(super) fn verify_signature(
    bundle_id: &str,
    format: BundleFormat,
    expires: i64,
    tenant_id: &str,
    presented: &str,
) -> bool {
    let expected = compute_signature(bundle_id, format, expires, tenant_id);
    constant_time_eq(expected.as_bytes(), presented.as_bytes())
}

fn compute_signature(
    bundle_id: &str,
    format: BundleFormat,
    expires: i64,
    tenant_id: &str,
) -> String {
    // mission-audit-export-tenant-scope-fix: bind the tenant into the
    // signed payload so a forged URL without the export secret fails
    // verification.
    let msg = format!("{bundle_id}|{}|{expires}|{tenant_id}", format_wire(format));
    hex::encode(hmac_sha256(&secret_bytes(), msg.as_bytes()))
}

fn secret_bytes() -> Vec<u8> {
    std::env::var(SECRET_ENV)
        .ok()
        .filter(|s| !s.is_empty())
        .map_or_else(|| DEV_FALLBACK_SECRET.to_vec(), String::into_bytes)
}

fn hmac_sha256(key: &[u8], msg: &[u8]) -> [u8; 32] {
    const BLOCK: usize = 64;
    let mut k = if key.len() > BLOCK {
        let mut h = Sha256::new();
        h.update(key);
        h.finalize().to_vec()
    } else {
        key.to_vec()
    };
    k.resize(BLOCK, 0);
    let opad: Vec<u8> = k.iter().map(|b| b ^ 0x5c).collect();
    let ipad: Vec<u8> = k.iter().map(|b| b ^ 0x36).collect();
    let mut inner = Sha256::new();
    inner.update(&ipad);
    inner.update(msg);
    let inner_digest = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(&opad);
    outer.update(inner_digest);
    let out = outer.finalize();
    let mut buf = [0u8; 32];
    buf.copy_from_slice(&out);
    buf
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

pub(super) fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

pub(super) fn export_root() -> PathBuf {
    if let Ok(dir) = std::env::var(EXPORT_DIR_ENV) {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    std::env::temp_dir().join("argo-audit-exports")
}

/// Per-tenant subdir under [`export_root`]. Returns `None` when
/// `tenant_id` could escape the subdir (path separators, traversal).
pub(super) fn tenant_export_dir(tenant_id: &str) -> Option<PathBuf> {
    if !is_safe_tenant_segment(tenant_id) {
        return None;
    }
    Some(export_root().join(tenant_id))
}

fn is_safe_tenant_segment(tenant_id: &str) -> bool {
    !tenant_id.is_empty()
        && tenant_id.len() <= 128
        && tenant_id != "."
        && tenant_id != ".."
        && tenant_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

pub(super) fn is_safe_bundle_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Extract the caller's tenant id from the `X-Argo-Tenant-Id`
/// header, falling back to [`DEFAULT_AUDIT_TENANT_ID`] when absent.
pub(super) fn request_tenant_id(req: &str) -> String {
    header_value(req, "x-argo-tenant-id").unwrap_or_else(|| DEFAULT_AUDIT_TENANT_ID.to_string())
}

fn header_value(req: &str, name_lower: &str) -> Option<String> {
    let headers = req
        .split_once("\r\n\r\n")
        .map_or(req, |(headers, _body)| headers);
    headers.lines().skip(1).find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.trim()
            .eq_ignore_ascii_case(name_lower)
            .then(|| value.trim().to_string())
            .filter(|s| !s.is_empty())
    })
}

/// Conservative percent-encoder for the `tenant` query parameter.
fn url_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for b in value.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Inverse of [`url_encode`]. Returns `None` on malformed escapes.
pub(super) fn url_decode(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                if i + 2 >= bytes.len() {
                    return None;
                }
                let hi = (bytes[i + 1] as char).to_digit(16)?;
                let lo = (bytes[i + 2] as char).to_digit(16)?;
                out.push(((hi << 4) | lo) as u8);
                i += 3;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

pub(super) fn query_kv(query: &str, key: &str) -> Option<String> {
    let trimmed = query.split_whitespace().next().unwrap_or(query);
    let qs = trimmed.split_once('?').map_or(trimmed, |(_, q)| q);
    qs.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == key).then(|| v.to_string())
    })
}
