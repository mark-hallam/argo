//! `argo daemon install` — render the launchd plist + write it to
//! `~/Library/LaunchAgents/com.skycot.argo.plist` + run `launchctl
//! load` on it. Stream `claude-installer-macos` (V1.4 macOS
//! deployment).
//!
//! All disk + process side-effects are gated behind the public
//! [`run`] entry point; the plist rendering + path resolution
//! lives in pure functions ([`render_plist`], [`resolve_paths`])
//! that the unit tests exercise without touching the real
//! filesystem.

use argo_types::{ArgoError, ArgoResult};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::Command;

const PLIST_TEMPLATE: &str = include_str!("../../../../installer/macos/com.skycot.argo.plist");

/// `Label` matches the `Label` key inside the plist; both
/// `launchctl load/unload` and `launchctl bootstrap/bootout` key
/// off it.
pub const PLIST_LABEL: &str = "com.skycot.argo";

/// File name of the plist inside `LaunchAgents/`.
pub const PLIST_FILENAME: &str = "com.skycot.argo.plist";

/// Pure function — substitute `{{ARGO_BIN}}`, `{{HOME}}`, and
/// `{{LOG_DIR}}` in the template. The template lives under
/// `installer/macos/` and is checked in so the binary is
/// self-contained (no runtime fetch).
pub fn render_plist(bin: &Path, home: &Path, log_dir: &Path) -> String {
    PLIST_TEMPLATE
        .replace("{{ARGO_BIN}}", &bin.display().to_string())
        .replace("{{HOME}}", &home.display().to_string())
        .replace("{{LOG_DIR}}", &log_dir.display().to_string())
}

/// Resolved set of paths needed for `daemon install/uninstall`.
/// Pulled out so the unit tests can exercise the resolution
/// logic without writing anything to disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPaths {
    /// Path to the `argo` binary (written into ProgramArguments).
    pub bin: PathBuf,
    /// `$HOME` (used for `WorkingDirectory` + `EnvironmentVariables.HOME`).
    pub home: PathBuf,
    /// `$HOME/Library/Logs` — where stdout/stderr go.
    pub log_dir: PathBuf,
    /// Where the rendered plist gets written.
    pub plist_path: PathBuf,
}

/// Resolve all paths the install flow needs from the given
/// overrides + `$HOME`. Pure function — no I/O.
pub fn resolve_paths(
    bin_override: Option<&str>,
    launch_agents_override: Option<&str>,
    home_env: Option<&str>,
    current_exe: &Path,
) -> ArgoResult<ResolvedPaths> {
    let home = match home_env {
        Some(h) if !h.is_empty() => PathBuf::from(h),
        _ => {
            return Err(ArgoError::Provider(
                "$HOME is unset; cannot resolve LaunchAgents directory".into(),
            ));
        }
    };
    let bin = match bin_override {
        Some(b) => PathBuf::from(b),
        None => current_exe.to_path_buf(),
    };
    let launch_agents_dir = match launch_agents_override {
        Some(p) => PathBuf::from(p),
        None => home.join("Library").join("LaunchAgents"),
    };
    let plist_path = launch_agents_dir.join(PLIST_FILENAME);
    let log_dir = home.join("Library").join("Logs");
    Ok(ResolvedPaths { bin, home, log_dir, plist_path })
}

/// Top-level handler.
pub async fn run(
    json: bool,
    dry_run: bool,
    bin_override: Option<&str>,
    launch_agents_override: Option<&str>,
) -> ArgoResult<()> {
    if !cfg!(target_os = "macos") {
        return Err(ArgoError::Provider(
            "argo daemon install is macOS-only (launchd is a macOS feature)".into(),
        ));
    }
    let current_exe = std::env::current_exe()
        .map_err(|e| ArgoError::Provider(format!("current_exe: {e}")))?;
    let home_env = std::env::var("HOME").ok();
    let paths = resolve_paths(
        bin_override,
        launch_agents_override,
        home_env.as_deref(),
        &current_exe,
    )?;
    let plist = render_plist(&paths.bin, &paths.home, &paths.log_dir);

    if dry_run {
        emit(&paths, false, json, Some(&plist))?;
        return Ok(());
    }

    if let Some(parent) = paths.plist_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            ArgoError::Io(format!("create_dir_all {}: {e}", parent.display()))
        })?;
    }
    std::fs::create_dir_all(&paths.log_dir).map_err(|e| {
        ArgoError::Io(format!("create_dir_all {}: {e}", paths.log_dir.display()))
    })?;
    std::fs::write(&paths.plist_path, &plist).map_err(|e| {
        ArgoError::Io(format!("write {}: {e}", paths.plist_path.display()))
    })?;

    let loaded = launchctl_load(&paths.plist_path)?;
    emit(&paths, loaded, json, None)?;
    Ok(())
}

fn launchctl_load(plist_path: &Path) -> ArgoResult<bool> {
    let _ = Command::new("launchctl").arg("unload").arg(plist_path).status();
    let status = Command::new("launchctl")
        .arg("load")
        .arg(plist_path)
        .status()
        .map_err(|e| ArgoError::Provider(format!("launchctl load: {e}")))?;
    Ok(status.success())
}

fn emit(paths: &ResolvedPaths, loaded: bool, json: bool, dry_run: Option<&str>) -> ArgoResult<()> {
    if json {
        let mut obj = json!({
            "label": PLIST_LABEL,
            "plist_path": paths.plist_path.display().to_string(),
            "bin": paths.bin.display().to_string(),
            "loaded": loaded,
            "dry_run": dry_run.is_some(),
        });
        if let Some(plist) = dry_run {
            obj["plist_body"] = json!(plist);
        }
        println!("{}", serde_json::to_string_pretty(&obj).unwrap_or_default());
    } else if let Some(plist) = dry_run {
        println!("argo daemon install (dry-run)");
        println!("  target: {}", paths.plist_path.display());
        println!("  bin:    {}", paths.bin.display());
        println!("---");
        println!("{plist}");
    } else {
        println!("argo daemon install");
        println!("  wrote:  {}", paths.plist_path.display());
        println!("  bin:    {}", paths.bin.display());
        println!("  loaded: {loaded}");
        if !loaded {
            println!(
                "  hint:   `launchctl load {}` failed; run it manually if needed",
                paths.plist_path.display()
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn render_plist_substitutes_placeholders() {
        let bin = Path::new("/usr/local/bin/argo");
        let home = Path::new("/Users/operator");
        let logs = Path::new("/Users/operator/Library/Logs");
        let out = render_plist(bin, home, logs);
        assert!(out.contains("<string>/usr/local/bin/argo</string>"));
        assert!(out.contains("<string>/Users/operator</string>"));
        assert!(out.contains("<string>/Users/operator/Library/Logs/argo.out.log</string>"));
        assert!(out.contains("<string>/Users/operator/Library/Logs/argo.err.log</string>"));
        assert!(!out.contains("{{ARGO_BIN}}"));
        assert!(!out.contains("{{HOME}}"));
        assert!(!out.contains("{{LOG_DIR}}"));
    }

    #[test]
    fn render_plist_keeps_static_keys() {
        let out = render_plist(
            Path::new("/x/argo"),
            Path::new("/x"),
            Path::new("/x/Library/Logs"),
        );
        assert!(out.contains("<key>Label</key>"));
        assert!(out.contains("<string>com.skycot.argo</string>"));
        assert!(out.contains("<key>RunAtLoad</key>"));
        assert!(out.contains("<key>KeepAlive</key>"));
    }

    #[test]
    fn resolve_paths_uses_home_default_dir() {
        let paths = resolve_paths(
            None,
            None,
            Some("/Users/operator"),
            Path::new("/usr/local/bin/argo"),
        )
        .expect("resolve_paths");
        assert_eq!(paths.bin, Path::new("/usr/local/bin/argo"));
        assert_eq!(paths.home, Path::new("/Users/operator"));
        assert_eq!(paths.log_dir, Path::new("/Users/operator/Library/Logs"));
        assert_eq!(
            paths.plist_path,
            Path::new("/Users/operator/Library/LaunchAgents/com.skycot.argo.plist")
        );
    }

    #[test]
    fn resolve_paths_respects_overrides() {
        let paths = resolve_paths(
            Some("/opt/argo/bin/argo"),
            Some("/tmp/fake-launch-agents"),
            Some("/Users/operator"),
            Path::new("/should/be/ignored"),
        )
        .expect("resolve_paths");
        assert_eq!(paths.bin, Path::new("/opt/argo/bin/argo"));
        assert_eq!(
            paths.plist_path,
            Path::new("/tmp/fake-launch-agents/com.skycot.argo.plist")
        );
    }

    #[test]
    fn resolve_paths_errors_when_home_missing() {
        let err = resolve_paths(None, None, None, Path::new("/x/argo")).unwrap_err();
        assert!(format!("{err}").contains("$HOME is unset"));
    }

    #[test]
    fn plist_filename_is_stable() {
        assert_eq!(PLIST_FILENAME, "com.skycot.argo.plist");
        assert_eq!(PLIST_LABEL, "com.skycot.argo");
    }
}
