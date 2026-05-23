//! Workspace credential master-key resolution and storage.
//!
//! Resolves the symmetric master key used by [`crate::age_wrap`] to
//! wrap individual credential values at rest.
//!
//! ## Two resolution surfaces
//!
//! - [`MasterKey::resolve`] — legacy **process-global** key. Source of
//!   the §3 P0 finding in eval-credentials-cross-tenant-isolation
//!   (round-9, PR #2080). Still exported for the legacy-fallback path
//!   in [`MasterKey::resolve_for_tenant`] and for the workspace-scoped
//!   `<workspace>/.argo/credentials.toml` file (which has no tenant
//!   axis). Do NOT introduce new callers in tenant-scoped flows.
//! - [`MasterKey::resolve_for_tenant`] — per-tenant master key derived
//!   via HKDF-SHA256 from [`WorkspaceRootKey`]. Replacement surface for
//!   the gateway/database credential encryption flow.
//!
//! ## Legacy precedence
//!
//! 1. `ARGO_CREDENTIALS_MASTER_KEY` env var (canonical age secret-key
//!    string, e.g. `AGE-SECRET-KEY-1...`).
//! 2. File at `~/.skycot/.master_key`, must be mode `0600` on Unix.
//! 3. Error — no silent default; the caller must opt in to plaintext
//!    operation explicitly.
//!
//! ## Workspace-root precedence (per-tenant flow)
//!
//! 1. `SKYCOT_WORKSPACE_ROOT_KEY_BASE64` env var (base64-encoded
//!    32 bytes). Fly ephemeral / CI runners / single-source rotation.
//! 2. Derived from the legacy [`MasterKey::resolve`] output via
//!    `HKDF-Expand(legacy_secret_bytes, "argo-workspace-root-v1")` for
//!    seamless upgrade — operators who only configured the legacy
//!    `ARGO_CREDENTIALS_MASTER_KEY` keep working; per-tenant keys
//!    derive deterministically from that one root.
//! 3. Error — same fail-closed contract as the legacy path.
//!
//! The `MasterKey` newtype redacts its inner contents in `Debug` output
//! so that accidental log lines or panic messages cannot leak the key.

use age::secrecy::ExposeSecret;
use age::x25519::Identity;
use argo_types::{ArgoError, ArgoResult, TenantId};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use bech32::{ToBase32, Variant};
use dashmap::DashMap;
use hkdf::Hkdf;
use sha2::Sha256;
use std::env::{self, VarError};
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, OnceLock};

/// Env var holding the canonical age secret-key string.
pub const MASTER_KEY_ENV: &str = "ARGO_CREDENTIALS_MASTER_KEY";

/// On-disk fallback path under the user home directory.
pub const MASTER_KEY_FILE_REL: &str = ".skycot/.master_key";

/// Env var holding the base64-encoded 32-byte workspace root key. Source
/// for HKDF derivation of every per-tenant master key.
pub const WORKSPACE_ROOT_KEY_ENV: &str = "SKYCOT_WORKSPACE_ROOT_KEY_BASE64";

/// Length of the [`WorkspaceRootKey`] in bytes (256 bits — matches the
/// SQLCipher / AEAD key sizes used elsewhere in the codebase).
pub const WORKSPACE_ROOT_KEY_LEN: usize = 32;

/// HKDF salt for [`WorkspaceRootKey::from_legacy_master_key`]. Versioned
/// so a future rotation primitive can derive a *different* root from the
/// same legacy secret without colliding.
const ROOT_FROM_LEGACY_INFO: &[u8] = b"argo-workspace-root-v1";

/// HKDF salt for per-tenant master-key derivation. Versioned for the
/// same reason — the `-v1` suffix lets a future migration use `-v2`
/// without colliding with on-disk ciphertexts that still validate
/// under `-v1`.
const TENANT_MASTER_KEY_INFO: &[u8] = b"argo-cred-master-v1";

/// Bech32 HRP for age secret-key strings (`AGE-SECRET-KEY-1...`).
const AGE_SECRET_KEY_HRP: &str = "age-secret-key-";

/// Symmetric master key for credential wrapping.
///
/// Internally an `age::x25519::Identity`; the corresponding public
/// recipient is derived on demand. Holds the secret in `age`'s
/// `SecretString`, which zeroes its buffer on drop.
#[derive(Clone)]
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

    /// Construct a `MasterKey` from 32 raw bytes. The bytes are
    /// bech32-encoded with the canonical age HRP so the resulting
    /// `Identity` round-trips through [`Self::as_age_secret`] /
    /// [`Self::from_age_secret`] unchanged.
    ///
    /// # Errors
    /// Returns [`ArgoError::Schema`] only in the (impossible) case that
    /// bech32 encoding or age parsing fails; both are infallible for a
    /// 32-byte buffer with a known-good HRP, but we surface the error
    /// rather than panicking to keep the public contract `Result`-typed.
    pub fn from_bytes(bytes: [u8; 32]) -> ArgoResult<Self> {
        let encoded = bech32::encode(AGE_SECRET_KEY_HRP, bytes.to_base32(), Variant::Bech32)
            .map_err(|e| ArgoError::Schema(format!("bech32 encode master key: {e}")))?;
        Self::from_age_secret(&encoded.to_uppercase())
    }

    /// Generate a fresh random master key. Intended for `argo
    /// credentials init` flows and tests.
    #[must_use]
    pub fn generate() -> Self {
        Self {
            identity: Identity::generate(),
        }
    }

    /// Resolve the **legacy process-global** master key per the
    /// documented precedence (env > file > error).
    ///
    /// `home_override` lets tests stand in for the user home; pass
    /// `None` to use [`dirs_home_dir`].
    ///
    /// New tenant-scoped callers MUST use [`Self::resolve_for_tenant`]
    /// instead; this resolver remains for the workspace-file flow and
    /// the lazy-migration fallback only.
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

    /// Resolve the **per-tenant** master key, deriving it via HKDF-SHA256
    /// from the [`WorkspaceRootKey`] and caching the result process-wide.
    ///
    /// The cache key is `tenant_id`; the value is an `Arc<MasterKey>`
    /// shared by every subsequent caller for the same tenant, so HKDF
    /// runs at most once per tenant per process boot. Tests that need a
    /// cache-cold call can use [`Self::derive_for_tenant`] directly.
    ///
    /// # Errors
    /// Returns whatever [`WorkspaceRootKey::resolve`] returns when the
    /// workspace-root source is missing or malformed; the per-tenant
    /// derivation itself is infallible once the root is in hand.
    pub fn resolve_for_tenant(tenant_id: TenantId) -> ArgoResult<Arc<Self>> {
        if let Some(cached) = tenant_key_cache().get(&tenant_id) {
            return Ok(Arc::clone(cached.value()));
        }
        let root = WorkspaceRootKey::resolve()?;
        let derived = Arc::new(Self::derive_for_tenant(&root, tenant_id)?);
        // `insert` may overwrite a concurrent racer's entry; that's
        // fine because every derivation produces an identical key.
        tenant_key_cache().insert(tenant_id, Arc::clone(&derived));
        Ok(derived)
    }

    /// Cache-bypass version of [`Self::resolve_for_tenant`]. Always
    /// runs HKDF; intended for tests that exercise determinism.
    ///
    /// # Errors
    /// Returns [`ArgoError::Schema`] only on the unreachable bech32 /
    /// age-parse failure path documented on [`Self::from_bytes`].
    pub fn derive_for_tenant(root: &WorkspaceRootKey, tenant_id: TenantId) -> ArgoResult<Self> {
        let mut output = [0u8; 32];
        let hk = Hkdf::<Sha256>::new(Some(TENANT_MASTER_KEY_INFO), &root.bytes);
        // The only failure mode for HKDF-Expand is requesting > 255 * Hash::OutputSize
        // bytes; 32 bytes from SHA-256 is well under that bound, so the
        // `.expect` is structurally safe (matches the brief verbatim).
        hk.expand(tenant_id.to_string().as_bytes(), &mut output)
            .expect("HKDF expand 32 bytes always fits");
        Self::from_bytes(output)
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
        f.debug_struct("MasterKey")
            .field("identity", &"<redacted>")
            .finish()
    }
}

/// 32-byte secret that seeds every per-tenant master key via HKDF.
///
/// Operator-managed: lives in [`WORKSPACE_ROOT_KEY_ENV`] (or, for
/// back-compat dev paths, is derived from the legacy
/// [`MASTER_KEY_ENV`] via HKDF so existing single-key deployments
/// keep working without re-onboarding).
pub struct WorkspaceRootKey {
    bytes: [u8; WORKSPACE_ROOT_KEY_LEN],
}

impl WorkspaceRootKey {
    /// Wrap a 32-byte buffer directly. Tests and explicit-init code
    /// paths only; production callers go through [`Self::resolve`].
    #[must_use]
    pub fn from_bytes(bytes: [u8; WORKSPACE_ROOT_KEY_LEN]) -> Self {
        Self { bytes }
    }

    /// Expose the raw 32 bytes. Internal use only — callers should
    /// prefer [`MasterKey::derive_for_tenant`] over consuming the raw
    /// bytes directly.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; WORKSPACE_ROOT_KEY_LEN] {
        &self.bytes
    }

    /// Resolve the workspace root key per the documented precedence
    /// (env var > derive-from-legacy > error).
    ///
    /// # Errors
    /// Returns [`ArgoError::Schema`] when neither source supplies a
    /// usable key, or when the supplied env value is malformed.
    pub fn resolve() -> ArgoResult<Self> {
        match env::var(WORKSPACE_ROOT_KEY_ENV) {
            Ok(raw) if !raw.trim().is_empty() => return Self::from_env_base64(&raw),
            Ok(_) | Err(VarError::NotPresent) => {}
            Err(VarError::NotUnicode(_)) => {
                return Err(ArgoError::Schema(format!(
                    "{WORKSPACE_ROOT_KEY_ENV} is not valid UTF-8"
                )));
            }
        }
        // Back-compat: derive a stable workspace root from whatever the
        // legacy resolver returns. This is the dev-and-upgrade path —
        // operators who only set `ARGO_CREDENTIALS_MASTER_KEY` keep
        // working, and the derived root is deterministic so per-tenant
        // keys are stable across restarts.
        if let Ok(legacy) = MasterKey::resolve(None) {
            return Ok(Self::from_legacy_master_key(&legacy));
        }
        Err(ArgoError::Schema(format!(
            "no workspace root key: set {WORKSPACE_ROOT_KEY_ENV} (base64 32 bytes) \
             or configure {MASTER_KEY_ENV} for back-compat derivation"
        )))
    }

    fn from_env_base64(raw: &str) -> ArgoResult<Self> {
        let decoded = B64
            .decode(raw.trim())
            .map_err(|e| ArgoError::Schema(format!("{WORKSPACE_ROOT_KEY_ENV} base64: {e}")))?;
        if decoded.len() != WORKSPACE_ROOT_KEY_LEN {
            return Err(ArgoError::Schema(format!(
                "{WORKSPACE_ROOT_KEY_ENV} wrong length: got {} bytes, want {}",
                decoded.len(),
                WORKSPACE_ROOT_KEY_LEN
            )));
        }
        let mut bytes = [0u8; WORKSPACE_ROOT_KEY_LEN];
        bytes.copy_from_slice(&decoded);
        Ok(Self { bytes })
    }

    /// Deterministically derive a workspace root from the legacy
    /// process-global master key. Lets pre-existing deployments
    /// continue running without operator action — the per-tenant
    /// derivation reads the legacy secret, mixes it through HKDF, and
    /// uses the output as a 32-byte root.
    #[must_use]
    pub fn from_legacy_master_key(legacy: &MasterKey) -> Self {
        let secret = legacy.as_age_secret();
        let hk = Hkdf::<Sha256>::new(Some(ROOT_FROM_LEGACY_INFO), secret.as_bytes());
        let mut bytes = [0u8; WORKSPACE_ROOT_KEY_LEN];
        hk.expand(b"workspace-root", &mut bytes)
            .expect("HKDF expand 32 bytes always fits");
        Self { bytes }
    }
}

impl std::fmt::Debug for WorkspaceRootKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkspaceRootKey")
            .field("bytes", &"<redacted>")
            .finish()
    }
}

/// Best-effort `$HOME` discovery without pulling in the `dirs` crate.
#[must_use]
pub fn dirs_home_dir() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from)
}

#[cfg(test)]
#[path = "master_key_tests.rs"]
mod master_key_tests;

/// Process-wide cache of per-tenant master keys. HKDF derivation is
/// ~50µs but the credential read path is hot — caching keeps it
/// O(DashMap lookup) after the first call per tenant.
fn tenant_key_cache() -> &'static DashMap<TenantId, Arc<MasterKey>> {
    static CACHE: OnceLock<DashMap<TenantId, Arc<MasterKey>>> = OnceLock::new();
    CACHE.get_or_init(DashMap::new)
}

/// Test-only escape hatch to clear the per-tenant cache between cases
/// that mutate the workspace-root env. Production code never calls this.
#[doc(hidden)]
pub fn _clear_tenant_key_cache_for_tests() {
    tenant_key_cache().clear();
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
