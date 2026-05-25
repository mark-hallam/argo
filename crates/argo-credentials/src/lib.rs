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
//! for backward compatibility; the same read rewrites them to encrypted
//! wrapped entries before returning:
//!
//! ```toml
//! [credentials]
//! SLACK_BOT_TOKEN = "xoxb-…"
//! ```
//!
//! [`migrate_plaintext_in_place`] exposes the same rewrite as an
//! explicit operator command; subsequent reads return only encrypted
//! entries.

mod age_wrap;
mod master_key;

#[cfg(test)]
#[path = "lib_tests.rs"]
mod lib_tests;

pub use age_wrap::{
    decrypt_value, decrypt_value_with_fallback, encrypt_value, DecryptedValue, KeyVersion,
};
pub use master_key::{
    dirs_home_dir, MasterKey, WorkspaceRootKey, MASTER_KEY_ENV, MASTER_KEY_FILE_REL,
    WORKSPACE_ROOT_KEY_ENV, WORKSPACE_ROOT_KEY_LEN,
};

#[doc(hidden)]
pub use master_key::_clear_tenant_key_cache_for_tests;

use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

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

/// Read and decrypt a credentials file. Plaintext entries are returned
/// unchanged for this read, then opportunistically rewritten in the
/// existing encrypted wrapped-entry format. Encrypted entries are
/// unwrapped with `key` and left byte-for-byte preserved unless the file
/// also contained plaintext entries.
///
/// Returns an empty map when the file does not exist. The caller can
/// rely on that for "no credentials configured yet" flows without
/// pre-checking `path.exists()`.
///
/// # Errors
/// Returns [`ArgoError::Io`] on read failure, [`ArgoError::Schema`]
/// on malformed TOML or base64, [`ArgoError::Provider`] when an
/// encrypted entry fails to unwrap (wrong master key, corrupted blob),
/// and [`ArgoError::Io`] when a plaintext migration rewrite fails.
pub fn read_encrypted(path: &Path, key: &MasterKey) -> ArgoResult<CredentialMap> {
    if !path.exists() {
        return Ok(CredentialMap::new());
    }
    let body = fs::read_to_string(path)
        .map_err(|e| ArgoError::Io(format!("read {}: {e}", path.display())))?;
    let document: EncryptedDocument = toml::from_str(&body)
        .map_err(|e| ArgoError::Schema(format!("parse {}: {e}", path.display())))?;
    let mut out = CredentialMap::new();
    let mut next: BTreeMap<String, EncryptedEntry> = BTreeMap::new();
    let mut should_migrate = false;
    for (k, entry) in document.credentials {
        match entry {
            EncryptedEntry::Plain(plain) => {
                let blob = age_wrap::encrypt_value(&plain, key)?;
                next.insert(k.clone(), EncryptedEntry::Wrapped { value: blob });
                out.insert(k, plain);
                should_migrate = true;
            }
            EncryptedEntry::Wrapped { value } => {
                let plain = age_wrap::decrypt_value(&value, key)?;
                next.insert(k.clone(), EncryptedEntry::Wrapped { value });
                out.insert(k, plain);
            }
        };
    }
    if should_migrate {
        let _ = warn_loose_perms(path)?;
        write_document(path, &next)?;
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

fn write_document(path: &Path, wrapped: &BTreeMap<String, EncryptedEntry>) -> ArgoResult<()> {
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
    write_body_atomically(path, body.as_bytes())?;
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|e| ArgoError::Io(format!("chmod 0600 {}: {e}", path.display())))?;
    Ok(())
}

fn write_body_atomically(path: &Path, body: &[u8]) -> ArgoResult<()> {
    for attempt in 0..16 {
        let tmp_path = temp_path_for(path, attempt);
        let mut options = OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut file = match options.open(&tmp_path) {
            Ok(file) => file,
            Err(err) if err.kind() == ErrorKind::AlreadyExists => continue,
            Err(err) => {
                return Err(ArgoError::Io(format!(
                    "open temp {}: {err}",
                    tmp_path.display()
                )));
            }
        };
        let write_result = (|| -> ArgoResult<()> {
            file.write_all(body)
                .map_err(|e| ArgoError::Io(format!("write {}: {e}", tmp_path.display())))?;
            file.sync_all()
                .map_err(|e| ArgoError::Io(format!("sync {}: {e}", tmp_path.display())))?;
            Ok(())
        })();
        drop(file);
        if let Err(err) = write_result {
            let _ = fs::remove_file(&tmp_path);
            return Err(err);
        }
        if let Err(err) = fs::rename(&tmp_path, path) {
            let _ = fs::remove_file(&tmp_path);
            return Err(ArgoError::Io(format!(
                "rename {} to {}: {err}",
                tmp_path.display(),
                path.display()
            )));
        }
        return Ok(());
    }
    Err(ArgoError::Io(format!(
        "create temp file for {}: too many name collisions",
        path.display()
    )))
}

fn temp_path_for(path: &Path, attempt: u8) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("credentials.toml");
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    parent.join(format!(
        ".{file_name}.{}.{}.{attempt}.tmp",
        std::process::id(),
        nanos
    ))
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
