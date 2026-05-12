//! Workspace credential master-key resolution and storage.
//!
//! Resolves the symmetric master key used by [`crate::age_wrap`] to
//! wrap individual credential values at rest. Precedence:
//!
//! 1. `ARGO_CREDENTIALS_MASTER_KEY` env var (canonical age secret-key
//!    string, e.g. `AGE-SECRET-KEY-1...`).
//! 2. File at `~/.skycot/.master_key`, must be mode `0600` on Unix.
//! 3. Error — no silent default; the caller must opt in to plaintext
//!    operation explicitly.
//!
//! The `MasterKey` newtype redacts its inner contents in `Debug` output
//! so that accidental log lines or panic messages cannot leak the key.

use age::secrecy::ExposeSecret;
use age::x25519::Identity;
use argo_types::{ArgoError, ArgoResult};
use std::env::{self, VarError};
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// Env var holding the canonical age secret-key string.
pub const MASTER_KEY_ENV: &str = "ARGO_CREDENTIALS_MASTER_KEY";

/// On-disk fallback path under the user home directory.
pub const MASTER_KEY_FILE_REL: &str = ".skycot/.master_key";

/// Symmetric master key for credential wrapping.
///
/// Internally an `age::x25519::Identity`; the corresponding public
/// recipient is derived on demand. Holds the secret in `age`'s
/// `SecretString`, which zeroes its buffer on drop.
pub struct MasterKey {
    identity: Identity,
}

impl MasterKey {
    /// Parse a canonical age secret-key string (`AGE-SECRET-KEY-1...`).
    ///
    /// # Errors
    /// Returns [`ArgoError::Schema`] when the string is not a valid age
    /// secret-key.
    pub fn from_age_secret(raw: &str) -> ArgoResult<Self> {
        let trimmed = raw.trim();
        let identity = Identity::from_str(trimmed)
            .map_err(|e| ArgoError::Schema(format!("invalid master key: {e}")))?;
        Ok(Self { identity })
    }

    /// Generate a fresh random master key. Intended for `argo
    /// credentials init` flows and tests.
    #[must_use]
    pub fn generate() -> Self {
        Self {
            identity: Identity::generate(),
        }
    }

    /// Resolve the master key per the documented precedence (env > file > error).
    ///
    /// `home_override` lets tests stand in for the user home; pass
    /// `None` to use [`dirs_home_dir`].
    ///
    /// # Errors
    /// Returns [`ArgoError::Schema`] when no source supplies a key, or
    /// when the supplied value is malformed.
    pub fn resolve(home_override: Option<&Path>) -> ArgoResult<Self> {
        match env::var(MASTER_KEY_ENV) {
            Ok(raw) if !raw.trim().is_empty() => return Self::from_age_secret(&raw),
            Ok(_) | Err(VarError::NotPresent) => {}
            Err(VarError::NotUnicode(_)) => {
                return Err(ArgoError::Schema(format!(
                    "{MASTER_KEY_ENV} is not valid UTF-8"
                )));
            }
        }
        if let Some(home) = home_override.map(Path::to_path_buf).or_else(dirs_home_dir) {
            let path = home.join(MASTER_KEY_FILE_REL);
            if path.exists() {
                warn_loose_perms(&path)?;
                let body = fs::read_to_string(&path).map_err(|e| {
                    ArgoError::Io(format!("read master key {}: {e}", path.display()))
                })?;
                return Self::from_age_secret(&body);
            }
        }
        Err(ArgoError::Schema(format!(
            "no credential master key: set {MASTER_KEY_ENV} or write {MASTER_KEY_FILE_REL} (mode 0600)"
        )))
    }

    /// Serialize back to the canonical age secret-key string. Intended
    /// for persistence flows like `argo credentials init` — never log.
    #[must_use]
    pub fn as_age_secret(&self) -> String {
        self.identity.to_string().expose_secret().to_string()
    }

    pub(crate) fn identity(&self) -> &Identity {
        &self.identity
    }

    pub(crate) fn recipient(&self) -> age::x25519::Recipient {
        self.identity.to_public()
    }
}

impl std::fmt::Debug for MasterKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MasterKey").field("identity", &"<redacted>").finish()
    }
}

/// Best-effort `$HOME` discovery without pulling in the `dirs` crate.
#[must_use]
pub fn dirs_home_dir() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from)
}

/// Emit a warning to stderr if the master-key file is more permissive
/// than `0600`. Non-Unix is a no-op (Windows ACLs are out of scope for
/// this brief).
fn warn_loose_perms(path: &Path) -> ArgoResult<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let meta = fs::metadata(path)
            .map_err(|e| ArgoError::Io(format!("stat {}: {e}", path.display())))?;
        let mode = meta.permissions().mode() & 0o777;
        if mode & 0o077 != 0 {
            eprintln!(
                "warning: {} is mode {:o}; tighten to 0600 (chmod 600 {0})",
                path.display(),
                mode
            );
        }
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}
