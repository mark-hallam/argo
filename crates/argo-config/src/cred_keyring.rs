//! OS-keyring backend for workspace credentials.
//!
//! Wraps the `keyring` crate v3 to provide a per-workspace credential
//! store backed by the platform-native secret manager (macOS Keychain,
//! Linux Secret Service, Windows Credential Manager). Each Argo
//! workspace is isolated under its own keyring service name —
//! `argo:<hash>`, where the hash is an 8-byte hex prefix of SHA-256
//! over the canonical workspace path. Operators with multiple Argo
//! workspaces on the same machine therefore see distinct keyring
//! entries even when the credential keys are identical.
//!
//! The migration tool (`argo cred migrate-to-keyring`) is the
//! supported path for moving file-backed credentials into the
//! keyring. Hot-path `get` calls dispatch through
//! [`crate::cred::get_with_backend`] which honors env-var precedence
//! and falls back from keyring to file when `Auto` is selected.

use keyring::Entry;
pub use keyring::Error as KeyringError;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::OnceLock;

const PROBE_SERVICE: &str = "argo:probe";
const PROBE_KEY: &str = "argo:probe:liveness";

/// Compute the keyring service name for a workspace.
///
/// The canonical path is hashed (SHA-256, 8-byte hex prefix) so two
/// workspaces with the same name in different parent directories map
/// to distinct services. Symlinks resolve before hashing.
fn service_name(workspace: &Path) -> String {
    let canonical = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf());
    let path_str = canonical.to_string_lossy();
    let mut hasher = Sha256::new();
    hasher.update(path_str.as_bytes());
    let digest = hasher.finalize();
    let hex: String = digest[..8].iter().map(|b| format!("{b:02x}")).collect();
    format!("argo:{hex}")
}

/// Read a credential from the OS keyring under the workspace's service.
///
/// # Errors
/// Returns [`KeyringError::NoEntry`] when the key does not exist, or a
/// platform error when the keyring backend itself fails.
pub fn keyring_get(workspace: &Path, key: &str) -> Result<String, KeyringError> {
    let entry = Entry::new(&service_name(workspace), key)?;
    entry.get_password()
}

/// Write a credential to the OS keyring under the workspace's service.
///
/// # Errors
/// Returns the platform-specific error when the keyring backend
/// rejects the write (e.g. user denied keychain access).
pub fn keyring_set(workspace: &Path, key: &str, value: &str) -> Result<(), KeyringError> {
    let entry = Entry::new(&service_name(workspace), key)?;
    entry.set_password(value)
}

/// Remove a credential from the OS keyring.
///
/// # Errors
/// Returns [`KeyringError::NoEntry`] when the key is already absent
/// (callers may treat this as success), or a platform error.
pub fn keyring_delete(workspace: &Path, key: &str) -> Result<(), KeyringError> {
    let entry = Entry::new(&service_name(workspace), key)?;
    entry.delete_credential()
}

/// List keys stored under the workspace's service — best-effort.
///
/// The `keyring` crate does not expose enumeration on any backend, so
/// this currently returns an empty `Vec`. Operators inspect keyring
/// contents via OS-native tooling (`security find-generic-password` on
/// macOS, `secret-tool` on Linux) — see the operator runbook
/// `docs/skycot/runbook/credentials-keyring.md`.
///
/// # Errors
/// Reserved for future backends that do support enumeration.
pub fn keyring_list(_workspace: &Path) -> Result<Vec<String>, KeyringError> {
    Ok(Vec::new())
}

/// Probe whether the OS keyring is usable on this machine.
///
/// Attempts a set / get / delete roundtrip under the dedicated probe
/// service `argo:probe`. Result is cached for the lifetime of the
/// process — a successful first probe will not be re-attempted, and a
/// failed probe is sticky. Headless CI containers without a Secret
/// Service / Keychain agent get `false`.
#[must_use]
pub fn keyring_available() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(probe)
}

fn probe() -> bool {
    let Ok(entry) = Entry::new(PROBE_SERVICE, PROBE_KEY) else {
        return false;
    };
    if entry.set_password("probe").is_err() {
        return false;
    }
    if entry.get_password().is_err() {
        let _ = entry.delete_credential();
        return false;
    }
    entry.delete_credential().is_ok()
}

#[cfg(test)]
#[path = "cred_keyring_tests.rs"]
mod tests;
