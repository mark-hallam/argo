//! Tenant-credential decrypt + lazy legacy->per-tenant re-encryption
//! migration. Split out of `cred/io.rs` to keep that file under the
//! 500-LOC ceiling (it crossed at the #2879+#2882 encrypt+lock merge).
//! Same visibility surface: `pub(super)`, consumed by `crate::cred`.
use super::io::with_locked_write;
use super::CredentialEntry;
use argo_credentials::{decrypt_value_with_fallback, encrypt_value, DecryptedValue, KeyVersion, MasterKey};
use argo_types::{ArgoError, ArgoResult, TenantId};
use std::collections::{BTreeMap, HashSet};
use std::sync::{Mutex, OnceLock};
use std::path::Path;

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

/// Decrypt a whole tenant section for bulk loaders ([`super::load_tenant`],
/// the hot path behind `resolve_provider_api_key`), using the **per-tenant**
/// master key with a legacy global-key fallback.
///
/// This mirrors the per-read [`decrypt_tenant_value_with_migration`] key
/// resolution so writes (encrypted under the per-tenant key by
/// [`set_tenant`]) and bulk reads agree on which key decrypts. Unlike the
/// per-read path it does NOT write back lazily-migrated blobs: bulk loaders
/// are read-only, and the `get`/`decrypt_tenant_value_with_migration` path
/// owns the lazy-migration write-back.
///
/// # Errors
/// Returns [`ArgoError`] when the per-tenant key cannot be resolved (and a
/// section entry is encrypted) or when a blob cannot be decrypted under
/// either the per-tenant or the legacy key.
pub(super) fn decrypt_tenant_section(
    tenant_id: &TenantId,
    section: BTreeMap<String, CredentialEntry>,
) -> ArgoResult<BTreeMap<String, String>> {
    let needs_key = section.values().any(CredentialEntry::is_encrypted);
    let primary = if needs_key {
        Some(MasterKey::resolve_for_tenant(*tenant_id)?)
    } else {
        None
    };
    let legacy = if needs_key {
        MasterKey::resolve(None).ok()
    } else {
        None
    };
    let mut out = BTreeMap::new();
    for (key, entry) in section {
        let value = match entry {
            CredentialEntry::Plain(v) => v,
            CredentialEntry::Encrypted { value } => {
                // `needs_key` is set whenever any entry is encrypted, so
                // `primary` is always `Some` on this arm.
                let primary = primary.as_ref().ok_or_else(|| {
                    ArgoError::Provider(
                        "decrypt tenant credential: master key not configured".to_string(),
                    )
                })?;
                decrypt_value_with_fallback(&value, primary, legacy.as_ref())
                    .map(|d| d.plaintext)
                    .map_err(|e| ArgoError::Provider(format!("decrypt {key}: {e}")))?
            }
        };
        out.insert(key, value);
    }
    Ok(out)
}

fn rewrite_tenant_entry(
    workspace: &Path,
    tenant_id: &TenantId,
    key: &str,
    new_blob_b64: String,
) -> ArgoResult<()> {
    with_locked_write(workspace, |document| {
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
        Ok(())
    })
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

