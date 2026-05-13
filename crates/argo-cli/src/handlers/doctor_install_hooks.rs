//! `argo doctor install-hooks` — copy the pre-commit secret-scan
//! hook into `.git/hooks/pre-commit` (Mission
//! `mission-secret-scanning-pre-commit-hook`).
//!
//! Looks up the repo root via `git rev-parse --show-toplevel`,
//! resolves the canonical hook source at
//! `scripts/git-hooks/pre-commit-secret-scan.sh`, and copies it
//! into `.git/hooks/pre-commit` with `0o755` perms. Refuses to
//! overwrite an existing hook unless `--force` is passed — operator
//! may have local customisations.

use argo_types::{ArgoError, ArgoResult};
use std::path::{Path, PathBuf};
use std::process::Command;

const HOOK_SOURCE: &str = "scripts/git-hooks/pre-commit-secret-scan.sh";
const HOOK_TARGET: &str = ".git/hooks/pre-commit";

/// Result envelope for `--json` mode. Keeps consumers stable when
/// new fields are added (e.g. a future `dry_run` flag).
#[derive(Debug, serde::Serialize)]
pub struct InstallResult {
    pub source: String,
    pub target: String,
    pub installed: bool,
    pub skipped_reason: Option<String>,
}

pub fn run(force: bool, json: bool) -> ArgoResult<()> {
    let root = repo_root()?;
    let source = root.join(HOOK_SOURCE);
    let target = root.join(HOOK_TARGET);
    if !source.exists() {
        return Err(ArgoError::Provider(format!(
            "hook source missing: {} — run from a checkout that includes scripts/git-hooks/",
            source.display()
        )));
    }
    let mut result = InstallResult {
        source: source.display().to_string(),
        target: target.display().to_string(),
        installed: false,
        skipped_reason: None,
    };
    if target.exists() && !force {
        result.skipped_reason = Some(
            "target exists; pass --force to overwrite (preserves operator customisations by default)"
                .into(),
        );
    } else {
        copy_hook(&source, &target)?;
        result.installed = true;
    }
    emit(&result, json)
}

fn copy_hook(source: &Path, target: &Path) -> ArgoResult<()> {
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| ArgoError::Provider(format!("create hooks dir: {e}")))?;
    }
    std::fs::copy(source, target)
        .map_err(|e| ArgoError::Provider(format!("copy hook: {e}")))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(target, perms)
            .map_err(|e| ArgoError::Provider(format!("chmod hook: {e}")))?;
    }
    Ok(())
}

fn emit(result: &InstallResult, json: bool) -> ArgoResult<()> {
    if json {
        let body = serde_json::to_string(result)
            .map_err(|e| ArgoError::Provider(format!("serialize: {e}")))?;
        println!("{body}");
        return Ok(());
    }
    if result.installed {
        println!(
            "argo doctor install-hooks: installed {} → {}",
            result.source, result.target
        );
    } else if let Some(reason) = &result.skipped_reason {
        println!("argo doctor install-hooks: skipped — {reason}");
    }
    Ok(())
}

fn repo_root() -> ArgoResult<PathBuf> {
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| ArgoError::Provider(format!("spawn git: {e}")))?;
    if !out.status.success() {
        return Err(ArgoError::Provider(
            "not inside a git repository (`git rev-parse --show-toplevel` failed)".into(),
        ));
    }
    let s = String::from_utf8(out.stdout)
        .map_err(|e| ArgoError::Provider(format!("git output not utf8: {e}")))?;
    Ok(PathBuf::from(s.trim()))
}
