//! Workspace credential storage helpers.
//!
//! # Tenancy model (mission-cred-get-tenant-parameter)
//!
//! Reading a credential requires a [`argo_types::TenantId`]. The
//! storage layout is per-tenant TOML sections in `<workspace>/.argo/
//! credentials.toml`:
//!
//! ```toml
//! # canonical per-tenant section
//! [credentials.tenant.<tenant_id_uuid>]
//! SLACK_BOT_TOKEN = "xoxb-..."
//!
//! # legacy root-level section (back-compat fallback only)
//! [credentials]
//! SLACK_BOT_TOKEN = "xoxb-..."
//! ```
//!
//! Reads via [`get`] consult the requested tenant's section first.
//! Misses fall through to the legacy root-level section with a
//! `tracing::warn!` (one warning per (tenant, key) per process — the
//! fallback is meant as a migration ramp, not a steady state).
//!
//! [`get_unscoped`] is the additive escape hatch for genuine single-
//! tenant operations (CLI startup probes, `argo creds export` etc.).
//! Every call site is expected to carry a `// tenant-unscoped:
//! <reason>` comment justifying the scope. Cross-tenant exposure
//! through this entrypoint is by design — callers MUST hold tenant
//! authority themselves.

/// Per-tenant SQLCipher encryption-key resolution
/// (`mission-sqlcipher-2-keyring-wire`). API only — wired into
/// [`crate::store::SessionStore`] in PR 4.
pub mod db_key;

mod io;
pub use io::{
    clear, clear_all, export_lines, import_current_env, set, set_tenant,
};

use argo_types::{ArgoError, ArgoResult, TenantId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env::{self, VarError};
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Mutex, OnceLock};

const ARGO_DIR: &str = ".argo";
const CREDENTIALS_FILE: &str = "credentials.toml";
const ARGO_TOML: &str = "argo.toml";

/// Credential storage backend selector.
///
/// `Auto` (the default) prefers the OS keyring when one is available
/// AND has an entry for the requested key, otherwise falls back to
/// the workspace credentials file. Operators move keys into the
/// keyring with `argo cred migrate-to-keyring`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CredentialBackend {
    /// `<workspace>/.argo/credentials.toml` (back-compat default).
    File,
    /// OS keyring (macOS Keychain / Linux Secret Service / Windows
    /// Credential Manager).
    Keyring,
    /// Keyring when populated + available, else file.
    #[default]
    Auto,
}

impl CredentialBackend {
    /// Stable lowercase identifier used in `argo.toml` and CLI args.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Keyring => "keyring",
            Self::Auto => "auto",
        }
    }
}

impl FromStr for CredentialBackend {
    type Err = ArgoError;
    fn from_str(s: &str) -> Result<Self, ArgoError> {
        match s.trim().to_ascii_lowercase().as_str() {
            "file" => Ok(Self::File),
            "keyring" => Ok(Self::Keyring),
            "auto" => Ok(Self::Auto),
            other => Err(ArgoError::Provider(format!(
                "invalid credential backend '{other}': expected file, keyring, or auto"
            ))),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct CredentialDocument {
    #[serde(default)]
    pub(crate) credentials: CredentialSections,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct CredentialSections {
    /// Per-tenant credentials: `[credentials.tenant.<uuid>]`.
    ///
    /// Skipped on serialize when empty so older argo-credentials
    /// readers (which know only the legacy root-level shape) keep
    /// round-tripping cleanly through `migrate_plaintext_in_place`
    /// etc.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) tenant: BTreeMap<String, BTreeMap<String, CredentialEntry>>,
    /// Legacy root-level entries inlined under `[credentials]`.
    #[serde(flatten)]
    pub(crate) root: BTreeMap<String, CredentialEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum CredentialEntry {
    Plain(String),
    Encrypted { value: String },
}

impl CredentialEntry {
    fn is_encrypted(&self) -> bool {
        matches!(self, Self::Encrypted { .. })
    }

    fn into_plain(self, key: Option<&argo_credentials::MasterKey>) -> ArgoResult<String> {
        match self {
            Self::Plain(value) => Ok(value),
            Self::Encrypted { value } => {
                let key = key.ok_or_else(|| {
                    ArgoError::Provider("decrypt credential: master key not configured".to_string())
                })?;
                argo_credentials::decrypt_value(&value, key)
            }
        }
    }
}

/// Return the credentials file path for a workspace root.
#[must_use]
pub fn credentials_path(workspace: &Path) -> PathBuf {
    workspace.join(ARGO_DIR).join(CREDENTIALS_FILE)
}

/// Read a tenant-scoped credential by key.
///
/// Resolution order:
/// 1. Process environment (preserved for all backends).
/// 2. The tenant's section in `<cwd>/.argo/credentials.toml`.
/// 3. Legacy root-level entry, with `tracing::warn!` on first hit per
///    (tenant, key) pair.
///
/// # Errors
/// Returns [`VarError::NotPresent`] when no source has the key, or
/// [`VarError::NotUnicode`] when the environment value is not valid
/// Unicode.
pub fn get(tenant_id: &TenantId, key: &str) -> Result<String, VarError> {
    let cwd = std::env::current_dir().map_err(|_| VarError::NotPresent)?;
    get_from_workspace(&cwd, tenant_id, key)
}

/// Unscoped credential read — bypasses tenancy isolation.
///
/// Use ONLY when the calling context legitimately has no tenant
/// identity (CLI startup probes, single-tenant `argo` invocations,
/// gateway-process secrets like webhook HMAC). Every call site SHOULD
/// carry a `// tenant-unscoped: <reason>` comment.
///
/// # Errors
/// Returns [`VarError::NotPresent`] when no source has the key, or
/// [`VarError::NotUnicode`] when the environment value is not valid
/// Unicode.
pub fn get_unscoped(key: &str) -> Result<String, VarError> {
    let cwd = std::env::current_dir().map_err(|_| VarError::NotPresent)?;
    get_unscoped_from_workspace(&cwd, key)
}

/// Tenant-scoped read from an explicit workspace root.
///
/// # Errors
/// As for [`get`].
pub fn get_from_workspace(
    workspace: &Path,
    tenant_id: &TenantId,
    key: &str,
) -> Result<String, VarError> {
    let backend = read_configured_backend(workspace);
    get_with_backend(workspace, Some(tenant_id), key, backend)
}

/// Unscoped read from an explicit workspace root.
///
/// # Errors
/// As for [`get_unscoped`].
pub fn get_unscoped_from_workspace(workspace: &Path, key: &str) -> Result<String, VarError> {
    let backend = read_configured_backend(workspace);
    get_with_backend(workspace, None, key, backend)
}

/// Read a credential using an explicit backend choice.
///
/// `tenant_id = Some(_)` selects the tenant's section first and falls
/// through to the legacy root-level section with a `tracing::warn!`.
/// `tenant_id = None` reads only the legacy root-level section (used
/// by [`get_unscoped`]).
///
/// Env-var precedence is unchanged: a value in the process environment
/// is returned regardless of `backend`. For `Auto`, the keyring is
/// consulted only when [`crate::cred_keyring::keyring_available`]
/// returns true; missing keyring entries fall through to the file
/// backend.
///
/// # Errors
/// Returns [`VarError::NotPresent`] when no backend has the key, or
/// [`VarError::NotUnicode`] when the environment value is not valid
/// Unicode.
pub fn get_with_backend(
    workspace: &Path,
    tenant_id: Option<&TenantId>,
    key: &str,
    backend: CredentialBackend,
) -> Result<String, VarError> {
    if tenant_id.is_none() {
        tracing::warn!(
            target: "argo_config::cred",
            key = key,
            "tenant-unscoped credential read (caller bypassed tenancy isolation)"
        );
    }
    match env::var(key) {
        Ok(value) => return Ok(value),
        Err(VarError::NotUnicode(value)) => return Err(VarError::NotUnicode(value)),
        Err(VarError::NotPresent) => {}
    }
    match backend {
        CredentialBackend::File => load_from_file(workspace, tenant_id, key),
        CredentialBackend::Keyring => keyring_lookup(workspace, tenant_id, key),
        CredentialBackend::Auto => {
            if crate::cred_keyring::keyring_available() {
                if let Ok(value) = keyring_lookup(workspace, tenant_id, key) {
                    return Ok(value);
                }
            }
            load_from_file(workspace, tenant_id, key)
        }
    }
}

fn keyring_lookup(
    workspace: &Path,
    tenant_id: Option<&TenantId>,
    key: &str,
) -> Result<String, VarError> {
    if let Some(tid) = tenant_id {
        let scoped = format!("tenant/{tid}/{key}");
        if let Ok(value) = crate::cred_keyring::keyring_get(workspace, &scoped) {
            return Ok(value);
        }
        if let Ok(value) = crate::cred_keyring::keyring_get(workspace, key) {
            warn_legacy_fallback(tid, key, "keyring");
            return Ok(value);
        }
        Err(VarError::NotPresent)
    } else {
        crate::cred_keyring::keyring_get(workspace, key).map_err(|_| VarError::NotPresent)
    }
}

fn load_from_file(
    workspace: &Path,
    tenant_id: Option<&TenantId>,
    key: &str,
) -> Result<String, VarError> {
    let document = load_document(workspace).map_err(|_| VarError::NotPresent)?;
    if let Some(tid) = tenant_id {
        if let Some(entry) = document
            .credentials
            .tenant
            .get(&tid.to_string())
            .and_then(|section| section.get(key))
            .cloned()
        {
            return decrypt_tenant_entry(workspace, tid, key, entry);
        }
        if let Some(entry) = document.credentials.root.get(key).cloned() {
            warn_legacy_fallback(tid, key, "file");
            // Legacy root-level entries decrypt under the legacy global
            // key; they migrate when an operator runs the migration CLI,
            // not on every read.
            let legacy_key = legacy_master_key_for(&document)?;
            return entry
                .into_plain(legacy_key.as_ref())
                .map_err(|_| VarError::NotPresent);
        }
        Err(VarError::NotPresent)
    } else {
        let legacy_key = legacy_master_key_for(&document)?;
        document
            .credentials
            .root
            .get(key)
            .cloned()
            .ok_or(VarError::NotPresent)
            .and_then(|entry| {
                entry
                    .into_plain(legacy_key.as_ref())
                    .map_err(|_| VarError::NotPresent)
            })
    }
}

/// Decrypt one tenant-scoped entry, lazy-migrating from the legacy
/// process-global key to the per-tenant key when needed.
///
/// Plain entries pass through unchanged. Encrypted entries go through
/// `decrypt_value_with_fallback`; a `KeyVersion::Legacy` result triggers
/// an in-place re-encrypt + write-back and a one-shot `tracing::warn!`
/// per (tenant, key).
fn decrypt_tenant_entry(
    workspace: &Path,
    tenant_id: &TenantId,
    key: &str,
    entry: CredentialEntry,
) -> Result<String, VarError> {
    match entry {
        CredentialEntry::Plain(value) => Ok(value),
        CredentialEntry::Encrypted { value } => {
            io::decrypt_tenant_value_with_migration(workspace, tenant_id, key, &value)
                .map_err(|_| VarError::NotPresent)
        }
    }
}

/// Resolve the **legacy** process-global master key only when the
/// document still has root-level encrypted entries that need it for
/// decrypt.
///
/// Tenant-scoped reads resolve their per-tenant key inside
/// [`decrypt_tenant_entry`] (and [`io::decrypt_tenant_value_with_migration`])
/// rather than going through this helper — keeping the two paths
/// separate lets `decrypt_value_with_fallback` carry the legacy key in
/// as an explicit `Option` only when the lazy-migration path actually
/// needs it.
fn legacy_master_key_for(
    document: &CredentialDocument,
) -> Result<Option<argo_credentials::MasterKey>, VarError> {
    let needs_key = document
        .credentials
        .root
        .values()
        .any(CredentialEntry::is_encrypted);
    if needs_key {
        argo_credentials::MasterKey::resolve(None)
            .map(Some)
            .map_err(|_| VarError::NotPresent)
    } else {
        Ok(None)
    }
}

fn warn_legacy_fallback(tenant_id: &TenantId, key: &str, backend: &str) {
    static SEEN: OnceLock<Mutex<std::collections::HashSet<String>>> = OnceLock::new();
    let set = SEEN.get_or_init(|| Mutex::new(std::collections::HashSet::new()));
    let cache_key = format!("{tenant_id}|{key}|{backend}");
    if let Ok(mut guard) = set.lock() {
        if !guard.insert(cache_key) {
            return;
        }
    }
    tracing::warn!(
        target: "argo_config::cred",
        tenant_id = %tenant_id,
        key = key,
        backend = backend,
        "credential found via legacy root-level fallback; migrate with `argo creds migrate-to-tenant <tid>`"
    );
}

fn read_configured_backend(workspace: &Path) -> CredentialBackend {
    let path = workspace.join(ARGO_TOML);
    let Ok(body) = fs::read_to_string(&path) else {
        return CredentialBackend::Auto;
    };
    let Ok(parsed) = toml::from_str::<toml::Value>(&body) else {
        return CredentialBackend::Auto;
    };
    parsed
        .get("credentials")
        .and_then(|c| c.get("backend"))
        .and_then(|b| b.as_str())
        .and_then(|s| CredentialBackend::from_str(s).ok())
        .unwrap_or(CredentialBackend::Auto)
}

pub(crate) fn load_document(workspace: &Path) -> ArgoResult<CredentialDocument> {
    let path = credentials_path(workspace);
    if !path.exists() {
        return Ok(CredentialDocument::default());
    }
    let body = fs::read_to_string(&path)
        .map_err(|e| ArgoError::Io(format!("read {}: {e}", path.display())))?;
    toml::from_str(&body)
        .map_err(|e| ArgoError::Provider(format!("parse {}: {e}", path.display())))
}

/// Load all stored credentials from a workspace's legacy root-level
/// section (back-compat for export / migrate tooling).
///
/// # Errors
/// Returns an [`ArgoError`] when the file exists but cannot be read or
/// parsed, or when an encrypted entry cannot be decrypted.
pub fn load(workspace: &Path) -> ArgoResult<BTreeMap<String, String>> {
    let document = load_document(workspace)?;
    let master_key = if document
        .credentials
        .root
        .values()
        .any(CredentialEntry::is_encrypted)
    {
        Some(argo_credentials::MasterKey::resolve(None).map_err(|e| {
            ArgoError::Provider(format!(
                "decrypt credential: master key not configured: {e}"
            ))
        })?)
    } else {
        None
    };
    let mut credentials = BTreeMap::new();
    for (key, entry) in document.credentials.root {
        let value = entry
            .into_plain(master_key.as_ref())
            .map_err(|e| ArgoError::Provider(format!("decrypt {key}: {e}")))?;
        credentials.insert(key, value);
    }
    Ok(credentials)
}

/// Load stored credentials for a specific tenant section.
///
/// Returns the per-tenant section only (no legacy fallback). Used by
/// the `argo creds migrate-to-tenant` helper and the cross-tenant
/// isolation tests.
///
/// # Errors
/// Returns an [`ArgoError`] when the file exists but cannot be read or
/// parsed, or when an encrypted entry cannot be decrypted.
pub fn load_tenant(
    workspace: &Path,
    tenant_id: &TenantId,
) -> ArgoResult<BTreeMap<String, String>> {
    let document = load_document(workspace)?;
    let section = match document.credentials.tenant.get(&tenant_id.to_string()) {
        Some(section) => section.clone(),
        None => return Ok(BTreeMap::new()),
    };
    let master_key = if section.values().any(CredentialEntry::is_encrypted) {
        Some(argo_credentials::MasterKey::resolve(None).map_err(|e| {
            ArgoError::Provider(format!(
                "decrypt credential: master key not configured: {e}"
            ))
        })?)
    } else {
        None
    };
    let mut credentials = BTreeMap::new();
    for (key, entry) in section {
        let value = entry
            .into_plain(master_key.as_ref())
            .map_err(|e| ArgoError::Provider(format!("decrypt {key}: {e}")))?;
        credentials.insert(key, value);
    }
    Ok(credentials)
}

#[cfg(test)]
#[path = "cred_encryption_tests.rs"]
mod cred_encryption_tests;

#[cfg(test)]
#[path = "cred_tenant_tests.rs"]
mod cred_tenant_tests;
