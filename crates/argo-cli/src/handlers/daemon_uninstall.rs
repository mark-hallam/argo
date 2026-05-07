//! `argo daemon uninstall` — `launchctl unload` the agent + remove
//! the plist file. Idempotent. Companion to
//! [`super::daemon_install`].

use argo_types::{ArgoError, ArgoResult};
use serde_json::json;
use std::path::Path;
use std::process::Command;

use super::daemon_install::{resolve_paths, ResolvedPaths};

/// Top-level handler. Mirrors [`super::daemon_install::run`]'s
/// shape so both subcommands feel consistent on the operator side.
pub async fn run(
    json: bool,
    dry_run: bool,
    launch_agents_override: Option<&str>,
) -> ArgoResult<()> {
    if !cfg!(target_os = "macos") {
        return Err(ArgoError::Provider(
            "argo daemon uninstall is macOS-only (launchd is a macOS feature)".into(),
        ));
    }
    let current_exe = std::env::current_exe()
        .map_err(|e| ArgoError::Provider(format!("current_exe: {e}")))?;
    let home_env = std::env::var("HOME").ok();
    let paths = resolve_paths(
        None,
        launch_agents_override,
        home_env.as_deref(),
        &current_exe,
    )?;
    let outcome = if dry_run {
        Outcome { plist_existed: paths.plist_path.exists(), unloaded: false, removed: false }
    } else {
        execute(&paths.plist_path)?
    };
    emit(&paths, &outcome, json, dry_run)
}

#[derive(Debug, Clone, Copy)]
struct Outcome {
    plist_existed: bool,
    unloaded: bool,
    removed: bool,
}

fn execute(plist_path: &Path) -> ArgoResult<Outcome> {
    let plist_existed = plist_path.exists();
    let unloaded = if plist_existed {
        Command::new("launchctl")
            .arg("unload")
            .arg(plist_path)
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    } else {
        false
    };
    let removed = if plist_existed {
        match std::fs::remove_file(plist_path) {
            Ok(()) => true,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
            Err(e) => {
                return Err(ArgoError::Io(format!(
                    "remove {}: {e}",
                    plist_path.display()
                )));
            }
        }
    } else {
        false
    };
    Ok(Outcome { plist_existed, unloaded, removed })
}

fn emit(paths: &ResolvedPaths, outcome: &Outcome, json: bool, dry_run: bool) -> ArgoResult<()> {
    if json {
        let obj = json!({
            "plist_path": paths.plist_path.display().to_string(),
            "plist_existed": outcome.plist_existed,
            "unloaded": outcome.unloaded,
            "removed": outcome.removed,
            "dry_run": dry_run,
        });
        println!("{}", serde_json::to_string_pretty(&obj).unwrap_or_default());
    } else if dry_run {
        println!("argo daemon uninstall (dry-run)");
        println!("  target:   {}", paths.plist_path.display());
        println!("  exists:   {}", outcome.plist_existed);
    } else {
        println!("argo daemon uninstall");
        println!("  target:   {}", paths.plist_path.display());
        println!("  unloaded: {}", outcome.unloaded);
        println!("  removed:  {}", outcome.removed);
        if !outcome.plist_existed {
            println!("  note:     plist not present; nothing to do");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn write_dummy_plist(launch_dir: &Path) -> PathBuf {
        let plist = launch_dir.join("com.skycot.argo.plist");
        std::fs::write(&plist, "<plist/>").expect("write");
        plist
    }

    #[test]
    fn execute_removes_existing_plist() {
        let tmp = TempDir::new().expect("tempdir");
        let plist = write_dummy_plist(tmp.path());
        let outcome = execute(&plist).expect("execute");
        assert!(outcome.plist_existed);
        assert!(outcome.removed);
        assert!(!plist.exists(), "plist should be deleted");
    }

    #[test]
    fn execute_is_idempotent_when_missing() {
        let tmp = TempDir::new().expect("tempdir");
        let plist = tmp.path().join("does-not-exist.plist");
        let outcome = execute(&plist).expect("execute");
        assert!(!outcome.plist_existed);
        assert!(!outcome.removed);
        assert!(!outcome.unloaded);
    }
}
