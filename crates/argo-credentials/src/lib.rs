//! At-rest encryption for the workspace credentials file.
//!
//! Connector API keys and OAuth tokens live in
//! `<workspace>/.argo/credentials.toml` (and historically a global
//! `~/.skycot/credentials.toml`). Enterprise tenants need that file
//! protected against backup/snapshot exfiltration and accidental file
//! sharing. This crate wraps individual values with [`age`] under a
//! workspace master key — the protection is at-rest only; an attacker
//! with shell access still wins.
//!
//! ## File format
//!
//! Encrypted entries are tables containing a single `value` field:
//!
//! ```toml
//! [credentials.SLACK_BOT_TOKEN]
//! value = "<base64-encoded-age-blob>"
//! ```
//!
//! Legacy plaintext entries (bare strings) are still accepted on read
//! to keep single-tenant deployments compiling without forced
//! migration:
//!
//! ```toml
//! [credentials]
//! SLACK_BOT_TOKEN = "xoxb-…"
//! ```
//!
//! [`migrate_plaintext_in_place`] rewrites a mixed file so that every
//! entry is encrypted; subsequent reads return only encrypted entries.

mod age_wrap;
mod master_key;

#[cfg(test)]
#[path = "lib_tests.rs"]
mod lib_tests;

pub use age_wrap::{decrypt_value, encrypt_value};
pub use master_key::{dirs_home_dir, MasterKey, MASTER_KEY_ENV, MASTER_KEY_FILE_REL};

use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::Path;

/// Map of credential key → plaintext value.
pub type CredentialMap = BTreeMap<String, String>;

/// Report returned by [`migrate_plaintext_in_place`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MigrationReport {
    /// Number of entries that were plaintext and have now been wrapped.
    pub encrypted: usize,
    /// Number of entries that were already wrapped (left untouched).
    pub already_encrypted: usize,
    /// `true` when the on-disk file permissions were looser than
    /// `0600` at migration time (a warning was emitted to stderr).
    pub perms_warning: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct EncryptedDocument {
    #[serde(default)]
    credentials: BTreeMap<String, EncryptedEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum EncryptedEntry {
    /// `{ value = "<blob>" }` — the encrypted-at-rest case.
    Wrapped { value: String },
    /// `"plaintext"` — legacy entries from before this brief landed.
    Plain(String),
}

/// Read and decrypt a credentials file. Plaintext entries are passed
/// through unchanged, encrypted entries are unwrapped with `key`.
///
/// Returns an empty map when the file does not exist. The caller can
/// rely on that for "no credentials configured yet" flows without
/// pre-checking `path.exists()`.
///
/// # Errors
/// Returns [`ArgoError::Io`] on read failure, [`ArgoError::Schema`]
/// on malformed TOML or base64, [`ArgoError::Provider`] when an
/// encrypted entry fails to unwrap (wrong master key, corrupted blob).
pub fn read_encrypted(path: &Path, key: &MasterKey) -> ArgoResult<CredentialMap> {
    if !path.exists() {
        return Ok(CredentialMap::new());
    }
    let body = fs::read_to_string(path)
        .map_err(|e| ArgoError::Io(format!("read {}: {e}", path.display())))?;
    let document: EncryptedDocument = toml::from_str(&body)
        .map_err(|e| ArgoError::Schema(format!("parse {}: {e}", path.display())))?;
    let mut out = CredentialMap::new();
    for (k, entry) in document.credentials {
        let plain = match entry {
            EncryptedEntry::Plain(s) => s,
            EncryptedEntry::Wrapped { value } => age_wrap::decrypt_value(&value, key)?,
        };
        out.insert(k, plain);
    }
    Ok(out)
}

/// Encrypt every entry in `credentials` and write the file atomically
/// at `path`, with `0600` permissions on Unix.
///
/// # Errors
/// Returns [`ArgoError::Io`] on filesystem errors,
/// [`ArgoError::Provider`] when age wrapping fails.
pub fn write_encrypted(
    path: &Path,
    credentials: &CredentialMap,
    key: &MasterKey,
) -> ArgoResult<()> {
    let mut wrapped: BTreeMap<String, EncryptedEntry> = BTreeMap::new();
    for (k, v) in credentials {
        let blob = age_wrap::encrypt_value(v, key)?;
        wrapped.insert(k.clone(), EncryptedEntry::Wrapped { value: blob });
    }
    write_document(path, &wrapped)
}

/// Migrate any plaintext entries in `path` to encrypted form. Already-
/// encrypted entries are left untouched. Returns a report describing
/// what changed.
///
/// Emits a stderr warning when the file mode is looser than `0600`
/// before migration; the rewritten file is always written at `0600`.
///
/// # Errors
/// Returns [`ArgoError::Io`] on filesystem errors,
/// [`ArgoError::Schema`] on malformed TOML, [`ArgoError::Provider`]
/// when age wrapping fails.
pub fn migrate_plaintext_in_place(path: &Path, key: &MasterKey) -> ArgoResult<MigrationReport> {
    let perms_warning = warn_loose_perms(path)?;
    if !path.exists() {
        return Ok(MigrationReport {
            perms_warning,
            ..MigrationReport::default()
        });
    }
    let body = fs::read_to_string(path)
        .map_err(|e| ArgoError::Io(format!("read {}: {e}", path.display())))?;
    let document: EncryptedDocument = toml::from_str(&body)
        .map_err(|e| ArgoError::Schema(format!("parse {}: {e}", path.display())))?;
    let mut report = MigrationReport {
        perms_warning,
        ..MigrationReport::default()
    };
    let mut next: BTreeMap<String, EncryptedEntry> = BTreeMap::new();
    for (k, entry) in document.credentials {
        match entry {
            EncryptedEntry::Plain(plain) => {
                let blob = age_wrap::encrypt_value(&plain, key)?;
                next.insert(k, EncryptedEntry::Wrapped { value: blob });
                report.encrypted += 1;
            }
            EncryptedEntry::Wrapped { value } => {
                next.insert(k, EncryptedEntry::Wrapped { value });
                report.already_encrypted += 1;
            }
        }
    }
    write_document(path, &next)?;
    Ok(report)
}

fn write_document(
    path: &Path,
    wrapped: &BTreeMap<String, EncryptedEntry>,
) -> ArgoResult<()> {
    let parent = path
        .parent()
        .ok_or_else(|| ArgoError::Io(format!("invalid credentials path {}", path.display())))?;
    if !parent.as_os_str().is_empty() {
        fs::create_dir_all(parent)
            .map_err(|e| ArgoError::Io(format!("create {}: {e}", parent.display())))?;
    }
    let document = EncryptedDocument {
        credentials: wrapped.clone(),
    };
    let body = toml::to_string_pretty(&document)
        .map_err(|e| ArgoError::Schema(format!("serialize {}: {e}", path.display())))?;
    let mut options = OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options
        .open(path)
        .map_err(|e| ArgoError::Io(format!("open {}: {e}", path.display())))?;
    file.write_all(body.as_bytes())
        .map_err(|e| ArgoError::Io(format!("write {}: {e}", path.display())))?;
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|e| ArgoError::Io(format!("chmod 0600 {}: {e}", path.display())))?;
    Ok(())
}

fn warn_loose_perms(path: &Path) -> ArgoResult<bool> {
    #[cfg(unix)]
    {
        if !path.exists() {
            return Ok(false);
        }
        let meta = fs::metadata(path)
            .map_err(|e| ArgoError::Io(format!("stat {}: {e}", path.display())))?;
        let mode = meta.permissions().mode() & 0o777;
        if mode & 0o077 != 0 {
            eprintln!(
                "warning: {} is mode {:o}; tighten to 0600 (chmod 600 {0})",
                path.display(),
                mode
            );
            return Ok(true);
        }
        Ok(false)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(false)
    }
}
