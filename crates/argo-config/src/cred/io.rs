//! Credential file I/O helpers — set/clear/import/export, plus the
//! shared `write_document` + key-validation primitives.
//!
//! Split out of `cred.rs` to keep the public-API surface under the
//! 500-LOC per-file ceiling. Re-exported through `crate::cred`.

use super::{
    load_document, CredentialDocument, CredentialEntry, CredentialSections,
};
use argo_credentials::{
    decrypt_value_with_fallback, encrypt_value, DecryptedValue, KeyVersion, MasterKey,
};
use argo_types::{ArgoError, ArgoResult, TenantId};
use std::collections::{BTreeMap, HashSet};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::Path;
use std::sync::{Mutex, OnceLock};

/// Set one credential value in the workspace credentials file (legacy
/// root-level section).
///
/// # Errors
/// Returns an [`ArgoError`] when the key is invalid or the file cannot be written.
pub fn set(workspace: &Path, key: &str, value: &str) -> ArgoResult<()> {
    validate_key(key)?;
    let mut document = load_document(workspace)?;
    document
        .credentials
        .root
        .insert(key.to_string(), CredentialEntry::Plain(value.to_string()));
    write_document(workspace, &document)
}

/// Set one credential value scoped to a specific tenant.
///
/// # Errors
/// Returns an [`ArgoError`] when the key is invalid or the file cannot
/// be written.
pub fn set_tenant(
    workspace: &Path,
    tenant_id: &TenantId,
    key: &str,
    value: &str,
) -> ArgoResult<()> {
    validate_key(key)?;
    let mut document = load_document(workspace)?;
    document
        .credentials
        .tenant
        .entry(tenant_id.to_string())
        .or_default()
        .insert(key.to_string(), CredentialEntry::Plain(value.to_string()));
    write_document(workspace, &document)
}

/// Remove one credential value (legacy root-level). Returns `true` when the key existed.
///
/// # Errors
/// Returns an [`ArgoError`] when the file cannot be read or written.
pub fn clear(workspace: &Path, key: &str) -> ArgoResult<bool> {
    validate_key(key)?;
    let mut document = load_document(workspace)?;
    let removed = document.credentials.root.remove(key).is_some();
    write_document(workspace, &document)?;
    Ok(removed)
}

/// Remove all stored credentials and return the number removed
/// (counts every entry across legacy root + every tenant section).
///
/// # Errors
/// Returns an [`ArgoError`] when the file cannot be read or written.
pub fn clear_all(workspace: &Path) -> ArgoResult<usize> {
    let document = load_document(workspace)?;
    let root_count = document.credentials.root.len();
    let tenant_count: usize = document
        .credentials
        .tenant
        .values()
        .map(BTreeMap::len)
        .sum();
    let cleared = CredentialDocument {
        credentials: CredentialSections::default(),
    };
    write_document(workspace, &cleared)?;
    Ok(root_count + tenant_count)
}

/// Import credential-looking environment variables into the workspace file
/// (legacy root-level section).
///
/// # Errors
/// Returns an [`ArgoError`] when the file cannot be read or written.
pub fn import_current_env(workspace: &Path) -> ArgoResult<usize> {
    let mut document = load_document(workspace)?;
    let mut imported = 0;
    for (key, value) in env::vars() {
        if looks_like_credential_key(&key) {
            validate_key(&key)?;
            document
                .credentials
                .root
                .insert(key, CredentialEntry::Plain(value));
            imported += 1;
        }
    }
    write_document(workspace, &document)?;
    Ok(imported)
}

/// Render stored credentials as shell export lines (legacy root-level).
///
/// # Errors
/// Returns an [`ArgoError`] when the file cannot be read or `shell` is unknown.
pub fn export_lines(workspace: &Path, shell: &str) -> ArgoResult<Vec<String>> {
    let credentials = super::load(workspace)?;
    let mut lines = Vec::with_capacity(credentials.len());
    for (key, value) in credentials {
        validate_key(&key)?;
        let line = match shell {
            "bash" | "zsh" => format!("export {key}={}", quote_posix(&value)),
            "fish" => format!("set -gx {key} {};", quote_posix(&value)),
            other => {
                return Err(ArgoError::Provider(format!(
                    "unsupported shell '{other}': expected bash, zsh, or fish"
                )));
            }
        };
        lines.push(line);
    }
    Ok(lines)
}

/// Decrypt an encrypted tenant-scoped credential entry, lazily migrating
/// it to the per-tenant master key when the blob was wrapped under the
/// legacy process-global key.
///
/// On a successful migration the tenant section is rewritten in place
/// with the re-encrypted value, and a one-shot `tracing::warn!` per
/// (tenant, key) records the upgrade so operators can observe lazy
/// migrations rolling forward in the log stream.
///
/// # Errors
/// Returns [`ArgoError`] when neither the per-tenant nor the legacy key
/// can decrypt the blob, when the per-tenant key cannot be resolved, or
/// when the re-encrypted document cannot be written back to disk.
pub(super) fn decrypt_tenant_value_with_migration(
    workspace: &Path,
    tenant_id: &TenantId,
    key: &str,
    blob_b64: &str,
) -> ArgoResult<String> {
    let primary = MasterKey::resolve_for_tenant(*tenant_id)?;
    // Legacy global key is best-effort: it only matters when the blob
    // was wrapped before the per-tenant migration shipped. If the
    // legacy resolver fails, the fallback path is simply unavailable
    // — that is the correct behavior for a fresh deployment.
    let legacy = MasterKey::resolve(None).ok();
    let DecryptedValue {
        plaintext,
        key_version,
    } = decrypt_value_with_fallback(blob_b64, primary.as_ref(), legacy.as_ref())?;
    if matches!(key_version, KeyVersion::Legacy) {
        // Re-wrap with the per-tenant key and overwrite the entry.
        let new_blob = encrypt_value(&plaintext, primary.as_ref())?;
        rewrite_tenant_entry(workspace, tenant_id, key, new_blob)?;
        warn_legacy_migration(tenant_id, key);
    }
    Ok(plaintext)
}

fn rewrite_tenant_entry(
    workspace: &Path,
    tenant_id: &TenantId,
    key: &str,
    new_blob_b64: String,
) -> ArgoResult<()> {
    let mut document = load_document(workspace)?;
    let section = document
        .credentials
        .tenant
        .entry(tenant_id.to_string())
        .or_default();
    section.insert(
        key.to_string(),
        CredentialEntry::Encrypted {
            value: new_blob_b64,
        },
    );
    write_document(workspace, &document)
}

fn warn_legacy_migration(tenant_id: &TenantId, key: &str) {
    static SEEN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let set = SEEN.get_or_init(|| Mutex::new(HashSet::new()));
    let cache_key = format!("{tenant_id}|{key}");
    if let Ok(mut guard) = set.lock() {
        if !guard.insert(cache_key) {
            return;
        }
    }
    tracing::warn!(
        target: "argo_config::cred",
        tenant_id = %tenant_id,
        key = key,
        "credential lazy-migrated from legacy process-global key to per-tenant key (key_version 1 → 2)"
    );
}

pub(super) fn write_document(workspace: &Path, document: &CredentialDocument) -> ArgoResult<()> {
    let path = super::credentials_path(workspace);
    let parent = path
        .parent()
        .ok_or_else(|| ArgoError::Schema(format!("invalid credentials path {}", path.display())))?;
    fs::create_dir_all(parent)
        .map_err(|e| ArgoError::Io(format!("create {}: {e}", parent.display())))?;
    let body = toml::to_string_pretty(document)
        .map_err(|e| ArgoError::Provider(format!("serialize {}: {e}", path.display())))?;
    let mut options = OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options
        .open(&path)
        .map_err(|e| ArgoError::Provider(format!("open {}: {e}", path.display())))?;
    file.write_all(body.as_bytes())
        .map_err(|e| ArgoError::Io(format!("write {}: {e}", path.display())))?;
    #[cfg(unix)]
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
        .map_err(|e| ArgoError::Provider(format!("chmod 0600 {}: {e}", path.display())))?;
    Ok(())
}

fn validate_key(key: &str) -> ArgoResult<()> {
    if is_valid_key(key) {
        Ok(())
    } else {
        Err(ArgoError::Provider(format!(
            "invalid credential key '{key}': expected shell-safe env var name"
        )))
    }
}

fn is_valid_key(key: &str) -> bool {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn looks_like_credential_key(key: &str) -> bool {
    is_valid_key(key)
        && (key.ends_with("_API_KEY")
            || key.ends_with("_TOKEN")
            || key.ends_with("_SECRET")
            || key.ends_with("_PASSWORD")
            || key.ends_with("_PAT")
            || key.contains("ACCESS_KEY"))
}

fn quote_posix(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
