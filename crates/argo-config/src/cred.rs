//! Workspace credential storage helpers.

use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env::{self, VarError};
use std::fs::{self, OpenOptions};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::str::FromStr;

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
struct CredentialDocument {
    #[serde(default)]
    credentials: BTreeMap<String, String>,
}

/// Return the credentials file path for a workspace root.
#[must_use]
pub fn credentials_path(workspace: &Path) -> PathBuf {
    workspace.join(ARGO_DIR).join(CREDENTIALS_FILE)
}

/// Read a credential by key, preserving normal environment variable precedence.
///
/// If `key` is present in the process environment, that value is returned.
/// Otherwise the helper falls back to `<cwd>/.argo/credentials.toml`.
///
/// # Errors
/// Returns [`VarError::NotPresent`] when neither source has the key, or
/// [`VarError::NotUnicode`] when the environment value is not valid Unicode.
pub fn get(key: &str) -> Result<String, VarError> {
    let cwd = std::env::current_dir().map_err(|_| VarError::NotPresent)?;
    get_from_workspace(&cwd, key)
}

/// Read a credential by key from a specific workspace, after checking env first.
///
/// The backend is resolved from `<workspace>/argo.toml`
/// `[credentials] backend = "..."` (default `auto`). Env-var
/// precedence is preserved for every backend.
///
/// # Errors
/// Returns [`VarError::NotPresent`] when neither source has the key, or
/// [`VarError::NotUnicode`] when the environment value is not valid Unicode.
pub fn get_from_workspace(workspace: &Path, key: &str) -> Result<String, VarError> {
    let backend = read_configured_backend(workspace);
    get_with_backend(workspace, key, backend)
}

/// Read a credential using an explicit backend choice.
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
    key: &str,
    backend: CredentialBackend,
) -> Result<String, VarError> {
    match env::var(key) {
        Ok(value) => return Ok(value),
        Err(VarError::NotUnicode(value)) => return Err(VarError::NotUnicode(value)),
        Err(VarError::NotPresent) => {}
    }
    match backend {
        CredentialBackend::File => load_from_file(workspace, key),
        CredentialBackend::Keyring => crate::cred_keyring::keyring_get(workspace, key)
            .map_err(|_| VarError::NotPresent),
        CredentialBackend::Auto => {
            if crate::cred_keyring::keyring_available() {
                if let Ok(value) = crate::cred_keyring::keyring_get(workspace, key) {
                    return Ok(value);
                }
            }
            load_from_file(workspace, key)
        }
    }
}

fn load_from_file(workspace: &Path, key: &str) -> Result<String, VarError> {
    load(workspace)
        .ok()
        .and_then(|credentials| credentials.get(key).cloned())
        .ok_or(VarError::NotPresent)
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

/// Load all stored credentials from a workspace.
///
/// # Errors
/// Returns an [`ArgoError`] when the file exists but cannot be read or parsed.
pub fn load(workspace: &Path) -> ArgoResult<BTreeMap<String, String>> {
    let path = credentials_path(workspace);
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let body = fs::read_to_string(&path)
        .map_err(|e| ArgoError::Provider(format!("read {}: {e}", path.display())))?;
    let document: CredentialDocument = toml::from_str(&body)
        .map_err(|e| ArgoError::Provider(format!("parse {}: {e}", path.display())))?;
    Ok(document.credentials)
}

/// Set one credential value in the workspace credentials file.
///
/// # Errors
/// Returns an [`ArgoError`] when the key is invalid or the file cannot be written.
pub fn set(workspace: &Path, key: &str, value: &str) -> ArgoResult<()> {
    validate_key(key)?;
    let mut credentials = load(workspace)?;
    credentials.insert(key.to_string(), value.to_string());
    write_credentials(workspace, &credentials)
}

/// Remove one credential value. Returns `true` when the key existed.
///
/// # Errors
/// Returns an [`ArgoError`] when the file cannot be read or written.
pub fn clear(workspace: &Path, key: &str) -> ArgoResult<bool> {
    validate_key(key)?;
    let mut credentials = load(workspace)?;
    let removed = credentials.remove(key).is_some();
    write_credentials(workspace, &credentials)?;
    Ok(removed)
}

/// Remove all stored credentials and return the number removed.
///
/// # Errors
/// Returns an [`ArgoError`] when the file cannot be read or written.
pub fn clear_all(workspace: &Path) -> ArgoResult<usize> {
    let count = load(workspace)?.len();
    write_credentials(workspace, &BTreeMap::new())?;
    Ok(count)
}

/// Import credential-looking environment variables into the workspace file.
///
/// # Errors
/// Returns an [`ArgoError`] when the file cannot be read or written.
pub fn import_current_env(workspace: &Path) -> ArgoResult<usize> {
    let mut credentials = load(workspace)?;
    let mut imported = 0;
    for (key, value) in env::vars() {
        if looks_like_credential_key(&key) {
            validate_key(&key)?;
            credentials.insert(key, value);
            imported += 1;
        }
    }
    write_credentials(workspace, &credentials)?;
    Ok(imported)
}

/// Render stored credentials as shell export lines.
///
/// # Errors
/// Returns an [`ArgoError`] when the file cannot be read or `shell` is unknown.
pub fn export_lines(workspace: &Path, shell: &str) -> ArgoResult<Vec<String>> {
    let credentials = load(workspace)?;
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

fn write_credentials(workspace: &Path, credentials: &BTreeMap<String, String>) -> ArgoResult<()> {
    let path = credentials_path(workspace);
    let parent = path
        .parent()
        .ok_or_else(|| ArgoError::Schema(format!("invalid credentials path {}", path.display())))?;
    fs::create_dir_all(parent)
        .map_err(|e| ArgoError::Provider(format!("create {}: {e}", parent.display())))?;
    let document = CredentialDocument {
        credentials: credentials.clone(),
    };
    let body = toml::to_string_pretty(&document)
        .map_err(|e| ArgoError::Provider(format!("serialize {}: {e}", path.display())))?;
    let mut options = OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options
        .open(&path)
        .map_err(|e| ArgoError::Provider(format!("open {}: {e}", path.display())))?;
    file.write_all(body.as_bytes())
        .map_err(|e| ArgoError::Provider(format!("write {}: {e}", path.display())))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_load_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        set(dir.path(), "POSTMARK_API_KEY", "pm_test").unwrap();

        let loaded = load(dir.path()).unwrap();
        assert_eq!(loaded.get("POSTMARK_API_KEY").map(String::as_str), Some("pm_test"));
    }

    #[test]
    fn clear_removes_one_key() {
        let dir = tempfile::tempdir().unwrap();
        set(dir.path(), "A_TOKEN", "one").unwrap();
        set(dir.path(), "B_TOKEN", "two").unwrap();

        assert!(clear(dir.path(), "A_TOKEN").unwrap());
        let loaded = load(dir.path()).unwrap();
        assert!(!loaded.contains_key("A_TOKEN"));
        assert_eq!(loaded.get("B_TOKEN").map(String::as_str), Some("two"));
    }

    #[test]
    fn clear_all_reports_removed_count() {
        let dir = tempfile::tempdir().unwrap();
        set(dir.path(), "A_TOKEN", "one").unwrap();
        set(dir.path(), "B_TOKEN", "two").unwrap();

        assert_eq!(clear_all(dir.path()).unwrap(), 2);
        assert!(load(dir.path()).unwrap().is_empty());
    }

    #[test]
    fn get_prefers_environment_over_file() {
        let dir = tempfile::tempdir().unwrap();
        let key = "ARGO_TEST_CRED_ENV_FIRST_TOKEN";
        set(dir.path(), key, "file").unwrap();
        env::set_var(key, "env");

        assert_eq!(get_from_workspace(dir.path(), key).unwrap(), "env");
        env::remove_var(key);
    }

    #[test]
    fn get_falls_back_to_file_when_env_unset() {
        let dir = tempfile::tempdir().unwrap();
        let key = "ARGO_TEST_CRED_FILE_TOKEN";
        env::remove_var(key);
        set(dir.path(), key, "file").unwrap();

        assert_eq!(get_from_workspace(dir.path(), key).unwrap(), "file");
    }

    #[test]
    fn export_bash_escapes_single_quotes() {
        let dir = tempfile::tempdir().unwrap();
        set(dir.path(), "ANTHROPIC_API_KEY", "sk-'quoted'").unwrap();

        let lines = export_lines(dir.path(), "bash").unwrap();
        assert_eq!(lines, vec!["export ANTHROPIC_API_KEY='sk-'\\''quoted'\\'''"]);
    }

    #[cfg(unix)]
    #[test]
    fn credentials_file_mode_is_0600() {
        let dir = tempfile::tempdir().unwrap();
        set(dir.path(), "POSTMARK_API_KEY", "pm_test").unwrap();

        let mode = fs::metadata(credentials_path(dir.path()))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }
}
