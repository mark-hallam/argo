//! `argo cred encrypt-all` — migrate the workspace credentials file
//! to at-rest age encryption in place.
//!
//! Reads `<workspace>/.argo/credentials.toml`, wraps every plaintext
//! entry with the workspace master key, and rewrites the file. Already-
//! encrypted entries are passed through unchanged. The master key is
//! resolved per [`argo_credentials::MasterKey::resolve`]: env first,
//! then `~/.skycot/.master_key`, else error.

use argo_credentials::{migrate_plaintext_in_place, MasterKey};
use argo_types::{ArgoError, ArgoResult};
use std::path::{Path, PathBuf};

const ARGO_DIR: &str = ".argo";
const CREDENTIALS_FILE: &str = "credentials.toml";

/// Entry point for the `argo cred encrypt-all` subcommand.
///
/// `path_override` is intended for tests and unusual deployments
/// (e.g. a per-tenant credentials file outside the workspace
/// `.argo/` dir); production use should pass `None` and let the
/// handler derive the canonical path.
///
/// # Errors
/// Surfaces any error from master-key resolution, TOML parsing, or
/// age wrapping.
pub fn run(path_override: Option<PathBuf>, json: bool) -> ArgoResult<()> {
    let path = match path_override {
        Some(p) => p,
        None => default_credentials_path()?,
    };
    let key = MasterKey::resolve(None)?;
    let report = migrate_plaintext_in_place(&path, &key)?;
    if json {
        let body = serde_json::json!({
            "path": path.display().to_string(),
            "encrypted": report.encrypted,
            "already_encrypted": report.already_encrypted,
            "perms_warning": report.perms_warning,
        });
        println!("{body}");
    } else {
        println!(
            "encrypted {} entr{} at {} ({} already encrypted)",
            report.encrypted,
            if report.encrypted == 1 { "y" } else { "ies" },
            path.display(),
            report.already_encrypted
        );
        if report.perms_warning {
            eprintln!(
                "warning: credentials file was looser than 0600 before migration; \
                 file is now 0600 but check earlier copies / backups"
            );
        }
    }
    Ok(())
}

fn default_credentials_path() -> ArgoResult<PathBuf> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    Ok(credentials_path(&cwd))
}

fn credentials_path(workspace: &Path) -> PathBuf {
    workspace.join(ARGO_DIR).join(CREDENTIALS_FILE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::{Mutex, OnceLock};

    fn env_mutex() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn run_encrypts_plaintext_file_in_workspace() {
        let _guard = env_mutex().lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let argo_dir = dir.path().join(ARGO_DIR);
        fs::create_dir_all(&argo_dir).unwrap();
        let path = argo_dir.join(CREDENTIALS_FILE);
        fs::write(&path, "[credentials]\nTOKEN = \"plain\"\n").unwrap();

        let key = MasterKey::generate();
        let secret = key.as_age_secret();
        let prior = std::env::var(argo_credentials::MASTER_KEY_ENV).ok();
        std::env::set_var(argo_credentials::MASTER_KEY_ENV, &secret);

        run(Some(path.clone()), false).unwrap();

        let body = fs::read_to_string(&path).unwrap();
        assert!(!body.contains("plain"), "plaintext leaked: {body}");
        let loaded = argo_config::cred::load(dir.path()).unwrap();
        assert_eq!(loaded.get("TOKEN").map(String::as_str), Some("plain"));

        match prior {
            Some(v) => std::env::set_var(argo_credentials::MASTER_KEY_ENV, v),
            None => std::env::remove_var(argo_credentials::MASTER_KEY_ENV),
        }
    }
}
