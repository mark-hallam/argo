//! `argo cred backend <kind>` + `argo cred migrate-to-keyring`.
//!
//! Backend selection writes `[credentials] backend = "..."` to
//! `<workspace>/argo.toml`, preserving any other contents. Migration
//! is all-or-none: every key is first written to the OS keyring; on
//! any failure, already-written keyring entries are rolled back and
//! the credentials file is left intact.

use argo_config::cred::{self, credentials_path, CredentialBackend};
use argo_config::cred_keyring;
use argo_types::{ArgoError, ArgoResult};
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

const ARGO_TOML: &str = "argo.toml";

/// Run `argo cred backend <kind>`.
pub fn run_backend(backend: &str) -> ArgoResult<()> {
    let parsed = CredentialBackend::from_str(backend)?;
    let workspace = workspace_dir()?;
    let toml_path = workspace.join(ARGO_TOML);
    let body = fs::read_to_string(&toml_path).unwrap_or_default();
    let mut doc: toml::Value = if body.trim().is_empty() {
        toml::Value::Table(toml::map::Map::new())
    } else {
        toml::from_str(&body)
            .map_err(|e| ArgoError::Provider(format!("parse {}: {e}", toml_path.display())))?
    };
    let root = doc
        .as_table_mut()
        .ok_or_else(|| ArgoError::Provider(format!("{} is not a TOML table", toml_path.display())))?;
    let section = root
        .entry("credentials")
        .or_insert(toml::Value::Table(toml::map::Map::new()));
    let section_table = section
        .as_table_mut()
        .ok_or_else(|| ArgoError::Provider("[credentials] is not a table".to_string()))?;
    section_table.insert(
        "backend".to_string(),
        toml::Value::String(parsed.as_str().to_string()),
    );
    let serialized = toml::to_string_pretty(&doc)
        .map_err(|e| ArgoError::Provider(format!("serialize {}: {e}", toml_path.display())))?;
    fs::write(&toml_path, serialized)
        .map_err(|e| ArgoError::Io(format!("write {}: {e}", toml_path.display())))?;
    println!("backend set to {}", parsed.as_str());
    Ok(())
}

/// Run `argo cred migrate-to-keyring [--dry-run] [--from-file PATH]`.
pub fn run_migrate(dry_run: bool, from_file: Option<PathBuf>) -> ArgoResult<()> {
    let workspace = workspace_dir()?;
    let source = from_file.unwrap_or_else(|| credentials_path(&workspace));
    let credentials = read_credentials(&source)?;
    if credentials.is_empty() {
        println!("no credentials to migrate ({})", source.display());
        return Ok(());
    }
    if dry_run {
        println!("dry-run: would migrate {} key(s) from {}", credentials.len(), source.display());
        for key in credentials.keys() {
            println!("  {key}");
        }
        return Ok(());
    }
    if !cred_keyring::keyring_available() {
        return Err(ArgoError::Provider(
            "keyring backend not available on this system (install gnome-keyring or KWallet on Linux; macOS / Windows ship Keychain / Credential Manager natively)".to_string(),
        ));
    }
    let mut written: Vec<String> = Vec::with_capacity(credentials.len());
    for (key, value) in &credentials {
        match cred_keyring::keyring_set(&workspace, key, value) {
            Ok(()) => {
                println!("migrated {key}");
                written.push(key.clone());
            }
            Err(err) => {
                let rollback_msg = rollback(&workspace, &written);
                return Err(ArgoError::Provider(format!(
                    "migration aborted at {key}: {err}; {rollback_msg}; file {} left intact",
                    source.display()
                )));
            }
        }
    }
    fs::remove_file(&source)
        .map_err(|e| ArgoError::Io(format!("remove {}: {e}", source.display())))?;
    println!(
        "migration complete: {} key(s); removed {}",
        written.len(),
        source.display()
    );
    Ok(())
}

fn rollback(workspace: &Path, written: &[String]) -> String {
    let mut rolled = 0usize;
    for key in written {
        if cred_keyring::keyring_delete(workspace, key).is_ok() {
            rolled += 1;
        }
    }
    format!("rolled back {rolled}/{} keyring write(s)", written.len())
}

fn read_credentials(path: &Path) -> ArgoResult<std::collections::BTreeMap<String, String>> {
    if !path.exists() {
        return Ok(std::collections::BTreeMap::new());
    }
    let body = fs::read_to_string(path)
        .map_err(|e| ArgoError::Io(format!("read {}: {e}", path.display())))?;
    // Reuse cred::load if reading from the default location to keep
    // parse semantics identical; otherwise parse the file inline so
    // operators can point at backups (--from-file).
    if let Some(parent) = path.parent().and_then(|p| p.parent()) {
        if credentials_path(parent) == *path {
            return cred::load(parent);
        }
    }
    #[derive(serde::Deserialize)]
    struct Doc {
        #[serde(default)]
        credentials: std::collections::BTreeMap<String, String>,
    }
    let doc: Doc = toml::from_str(&body)
        .map_err(|e| ArgoError::Provider(format!("parse {}: {e}", path.display())))?;
    Ok(doc.credentials)
}

fn workspace_dir() -> ArgoResult<PathBuf> {
    std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))
}
