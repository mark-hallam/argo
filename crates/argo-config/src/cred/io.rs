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
use std::sync::{Mutex, OnceLock, PoisonError};

/// Process-wide serialization lock for credential-file mutations.
///
/// Every writer of `credentials.toml` performs a read-modify-write
/// (`load_document` → mutate → atomic replace). Without serialization two
/// concurrent writers can lost-update: both read the same on-disk state,
/// each adds its own section, and the second to write clobbers the first
/// (last-writer-wins, the first writer's just-saved credential silently
/// dropped). That path became concurrently reachable via the tg2 gateway
/// connector-authorize route.
///
/// The gateway is a single process, so every concurrent writer shares this
/// address space; a process-wide [`Mutex`] is sufficient to serialize them
/// and is simpler than a cross-process file lock (no other process writes
/// this file). Cross-process safety (`flock`) is deliberately out of scope.
fn write_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Run a read-modify-write against the credentials document while holding
/// the process-wide [`write_lock`], then atomically persist the result.
///
/// Holding the lock across the load and the write is what closes the
/// lost-update window — the whole sequence is exclusive, not just the
/// final write. A poisoned lock is recovered via
/// [`PoisonError::into_inner`]: the guarded data is `()`, so there is no
/// invariant a previous panic could have left inconsistent, and refusing
/// to write credentials forever after one unrelated panic would be the
/// worse failure.
fn with_locked_write<F, T>(workspace: &Path, mutate: F) -> ArgoResult<T>
where
    F: FnOnce(&mut CredentialDocument) -> ArgoResult<T>,
{
    let _guard = write_lock().lock().unwrap_or_else(PoisonError::into_inner);
    let mut document = load_document(workspace)?;
    let outcome = mutate(&mut document)?;
    write_document(workspace, &document)?;
    Ok(outcome)
}

/// Set one credential value in the workspace credentials file (legacy
/// root-level section).
///
/// # Errors
/// Returns an [`ArgoError`] when the key is invalid or the file cannot be written.
pub fn set(workspace: &Path, key: &str, value: &str) -> ArgoResult<()> {
    validate_key(key)?;
    with_locked_write(workspace, |document| {
        document
            .credentials
            .root
            .insert(key.to_string(), CredentialEntry::Plain(value.to_string()));
        Ok(())
    })
}

/// Set one credential value scoped to a specific tenant, **encrypted at
/// rest** under the per-tenant master key.
///
/// This is the write path behind BYOK provider keys
/// (`POST /api/v1/providers/keys`) and tenant connector tokens. The
/// stored value is wrapped as [`CredentialEntry::Encrypted`] whenever a
/// per-tenant master key resolves (the steady state in any configured
/// deployment). When no master key is configured — a key-less dev/test
/// workspace — it logs a one-shot warning and falls back to
/// [`CredentialEntry::Plain`] so those environments keep round-tripping.
/// The read path ([`super::get`] / [`super::load_tenant`]) decrypts both
/// shapes transparently, so callers never see the difference.
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
    // #2879 (encrypt-at-rest) + #2882 (locked atomic write), combined:
    // encrypt under the per-tenant master key OUTSIDE the lock (no I/O),
    // then mutate the document inside the serialized read-modify-write.
    let entry = encrypt_tenant_value(*tenant_id, value);
    with_locked_write(workspace, |document| {
        document
            .credentials
            .tenant
            .entry(tenant_id.to_string())
            .or_default()
            .insert(key.to_string(), entry);
        Ok(())
    })
}

/// Wrap a tenant credential for storage, encrypting at rest under the
/// per-tenant master key.
///
/// Returns [`CredentialEntry::Encrypted`] when
/// [`MasterKey::resolve_for_tenant`] succeeds. When no master key source
/// is configured (`ARGO_WORKSPACE_ROOT_KEY` / `ARGO_CREDENTIALS_MASTER_KEY`
/// both absent and no key file on disk), it emits a one-shot
/// `tracing::warn!` per tenant and falls back to
/// [`CredentialEntry::Plain`] so key-less workspaces keep working. This
/// mirrors the read path's existing tolerance of mixed plain/encrypted
/// entries — encrypt when we can, never break a write because no key is
/// configured, but make the downgrade observable.
fn encrypt_tenant_value(tenant_id: TenantId, value: &str) -> CredentialEntry {
    match MasterKey::resolve_for_tenant(tenant_id)
        .and_then(|key| encrypt_value(value, key.as_ref()))
    {
        Ok(blob) => CredentialEntry::Encrypted { value: blob },
        Err(_) => {
            warn_plaintext_fallback(tenant_id);
            CredentialEntry::Plain(value.to_string())
        }
    }
}

fn warn_plaintext_fallback(tenant_id: TenantId) {
    static SEEN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let set = SEEN.get_or_init(|| Mutex::new(HashSet::new()));
    if let Ok(mut guard) = set.lock() {
        if !guard.insert(tenant_id.to_string()) {
            return;
        }
    }
    tracing::warn!(
        target: "argo_config::cred",
        tenant_id = %tenant_id,
        "tenant credential stored UNENCRYPTED at rest: no master key configured \
         (set ARGO_WORKSPACE_ROOT_KEY or ARGO_CREDENTIALS_MASTER_KEY to encrypt)"
    );
}

/// Remove one credential value (legacy root-level). Returns `true` when the key existed.
///
/// # Errors
/// Returns an [`ArgoError`] when the file cannot be read or written.
pub fn clear(workspace: &Path, key: &str) -> ArgoResult<bool> {
    validate_key(key)?;
    with_locked_write(workspace, |document| {
        Ok(document.credentials.root.remove(key).is_some())
    })
}

/// Remove all stored credentials and return the number removed
/// (counts every entry across legacy root + every tenant section).
///
/// # Errors
/// Returns an [`ArgoError`] when the file cannot be read or written.
pub fn clear_all(workspace: &Path) -> ArgoResult<usize> {
    with_locked_write(workspace, |document| {
        let root_count = document.credentials.root.len();
        let tenant_count: usize = document
            .credentials
            .tenant
            .values()
            .map(BTreeMap::len)
            .sum();
        *document = CredentialDocument {
            credentials: CredentialSections::default(),
        };
        Ok(root_count + tenant_count)
    })
}

/// Import credential-looking environment variables into the workspace file
/// (legacy root-level section).
///
/// # Errors
/// Returns an [`ArgoError`] when the file cannot be read or written.
pub fn import_current_env(workspace: &Path) -> ArgoResult<usize> {
    with_locked_write(workspace, |document| {
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
        Ok(imported)
    })
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

/// Persist `document` to the credentials file via write-temp-then-rename.
///
/// The body is written to a sibling `*.tmp` file (created 0600), flushed to
/// disk with [`File::sync_all`], then [`fs::rename`]d over the real path.
/// `rename(2)` is atomic within a filesystem, so a crash mid-write leaves
/// either the previous complete file or the new complete file on disk —
/// never a torn/partial one. The temp lives in the same directory as the
/// target so the rename stays intra-filesystem.
///
/// Callers reach this only through [`with_locked_write`], i.e. while holding
/// the process-wide [`write_lock`], so the fixed temp name cannot collide
/// between concurrent writers.
pub(super) fn write_document(workspace: &Path, document: &CredentialDocument) -> ArgoResult<()> {
    let path = super::credentials_path(workspace);
    let parent = path
        .parent()
        .ok_or_else(|| ArgoError::Schema(format!("invalid credentials path {}", path.display())))?;
    fs::create_dir_all(parent)
        .map_err(|e| ArgoError::Io(format!("create {}: {e}", parent.display())))?;
    let body = toml::to_string_pretty(document)
        .map_err(|e| ArgoError::Provider(format!("serialize {}: {e}", path.display())))?;

    // Stage into a sibling temp file, then atomically rename into place.
    let tmp_path = path.with_extension("toml.tmp");
    let mut options = OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options
        .open(&tmp_path)
        .map_err(|e| ArgoError::Provider(format!("open {}: {e}", tmp_path.display())))?;
    file.write_all(body.as_bytes())
        .map_err(|e| ArgoError::Io(format!("write {}: {e}", tmp_path.display())))?;
    file.sync_all()
        .map_err(|e| ArgoError::Io(format!("sync {}: {e}", tmp_path.display())))?;
    #[cfg(unix)]
    fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o600))
        .map_err(|e| ArgoError::Provider(format!("chmod 0600 {}: {e}", tmp_path.display())))?;
    fs::rename(&tmp_path, &path).map_err(|e| {
        // Best-effort cleanup so a failed rename does not leak the temp file.
        let _ = fs::remove_file(&tmp_path);
        ArgoError::Io(format!(
            "rename {} -> {}: {e}",
            tmp_path.display(),
            path.display()
        ))
    })?;
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

#[cfg(test)]
mod write_lock_tests {
    //! Brief: `launch-cred-file-write-lock` — durability of the
    //! credentials file under the concurrent tg2 gateway HTTP surface.
    use super::*;
    use argo_types::TenantId;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    /// Acceptance 1: N parallel `set_tenant` writers — every section is
    /// present afterwards. Pre-fix this flakes/fails: the unlocked
    /// load-modify-write lets two writers read the same state and the
    /// second clobbers the first (last-writer-wins).
    #[test]
    fn concurrent_set_tenant_keeps_every_section() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        const N: usize = 24;
        let tenants: Vec<TenantId> = (0..N).map(|_| TenantId::new()).collect();

        let handles: Vec<_> = tenants
            .iter()
            .copied()
            .map(|tenant| {
                let root = root.clone();
                std::thread::spawn(move || {
                    set_tenant(&root, &tenant, "SLACK_BOT_TOKEN", "xoxb-token").unwrap();
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }

        let document = load_document(&root).unwrap();
        assert_eq!(
            document.credentials.tenant.len(),
            N,
            "lost-update dropped at least one tenant section"
        );
        for tenant in &tenants {
            assert!(
                document
                    .credentials
                    .tenant
                    .contains_key(&tenant.to_string()),
                "missing section for tenant {tenant}"
            );
        }
    }

    /// Acceptance 2: a concurrent reader never observes a torn file. With
    /// temp-write + rename every snapshot the OS hands back is a complete,
    /// parseable document (the old one or the new one), never a partial
    /// truncate-in-place write. Also asserts no staging file lingers.
    #[test]
    fn atomic_write_never_exposes_a_torn_file() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let seed = TenantId::new();
        set_tenant(&root, &seed, "SEED_TOKEN", "seed").unwrap();

        let stop = Arc::new(AtomicBool::new(false));
        let reader_root = root.clone();
        let reader_stop = Arc::clone(&stop);
        let reader = std::thread::spawn(move || {
            let path = crate::cred::credentials_path(&reader_root);
            while !reader_stop.load(Ordering::Relaxed) {
                if let Ok(body) = fs::read_to_string(&path) {
                    toml::from_str::<CredentialDocument>(&body)
                        .expect("reader observed a torn / partially-written credentials file");
                }
            }
        });

        for i in 0..200 {
            let tenant = TenantId::new();
            set_tenant(&root, &tenant, "ROTATE_TOKEN", &format!("v{i}")).unwrap();
        }
        stop.store(true, Ordering::Relaxed);
        reader.join().unwrap();

        let tmp = crate::cred::credentials_path(&root).with_extension("toml.tmp");
        assert!(!tmp.exists(), "temp staging file leaked: {}", tmp.display());
        let document = load_document(&root).unwrap();
        assert!(
            document.credentials.tenant.contains_key(&seed.to_string()),
            "seed section lost after concurrent rotation"
        );
    }
}
