//! `argo db rekey` — rotate the at-rest encryption key in place
//! (Stream `mission-encryption-at-rest-sqlcipher`).
//!
//! Loads the *current* key from `[security.encryption] key_source` in
//! `argo.toml`, opens the store with `PRAGMA key` applied, then issues
//! `PRAGMA rekey` with the new key. Both keys go through
//! [`argo_store::encryption::KeySource::parse`] for validation; the
//! key values themselves never leave their source (env / file).

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_config::ArgoConfig;
use argo_store::encryption::{apply_rekey_pragma, KeySource};
use argo_store::{wal_tuning::WalTuningProfile, SessionStore};
use argo_types::{ArgoError, ArgoResult};

/// Run `argo db rekey`. Resolves old + new key sources and applies
/// `PRAGMA rekey` against the workspace store.
///
/// # Errors
/// - workspace home unresolved or unreadable
/// - either key source fails validation / resolution
/// - SQLite rejects the rekey statement
pub async fn run(new_key_source_spec: &str, json: bool) -> ArgoResult<()> {
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let db = db_path(&home);

    // Resolve the *current* key from argo.toml. If the workspace
    // has never enabled encryption, this is `KeySource::None`.
    let argo_toml = home.join("argo.toml");
    let cfg = ArgoConfig::load(&argo_toml)?;
    let old_key_source = KeySource::parse(&cfg.security.encryption.key_source)?;
    let new_key_source = KeySource::parse(new_key_source_spec)?;

    // Force-resolve both up front so we fail before touching the DB
    // when either source is broken (file missing, bad hex, etc.).
    let _ = old_key_source.resolve()?;
    let new_resolved = new_key_source.resolve()?;

    let Some(new_hex) = new_resolved else {
        return Err(ArgoError::Provider(
            "argo db rekey requires a non-none --new-key-source; \
             disabling encryption is a separate workflow"
                .into(),
        ));
    };

    let store =
        SessionStore::open_with_encryption(&db, WalTuningProfile::default(), &old_key_source)
            .await?;
    apply_rekey_pragma(store.pool(), &new_hex).await?;

    if json {
        let payload = serde_json::json!({
            "ok": true,
            "state": "rotated",
            "new_key_source": new_key_source_spec,
        });
        println!("{}", serde_json::to_string(&payload).unwrap_or_default());
    } else {
        println!("db: rekey rotated");
        println!("  new key_source: {new_key_source_spec}");
        println!("  warning: losing the key means losing the data — back it up.");
    }

    Ok(())
}
