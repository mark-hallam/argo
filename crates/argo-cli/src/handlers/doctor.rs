//! `argo doctor` — config / workspace sanity report.
use crate::home::resolve_home;
use argo_config::ArgoConfig;
use argo_types::{ArgoError, ArgoResult};

pub fn run() -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let toml = cwd.join("argo.toml");
    let toml_status = if toml.exists() { "OK" } else { "MISSING" };
    let cfg = ArgoConfig::load(&toml).unwrap_or_default();
    let home = resolve_home(&cwd).unwrap_or_else(|_| cfg.core.resolved_home());
    let home_status = if home.exists() {
        "OK"
    } else {
        "MISSING (will be created on first run)"
    };
    println!("argo doctor — workspace check");
    println!("  cwd:        {}", cwd.display());
    println!("  argo.toml:  {} ({})", toml.display(), toml_status);
    println!("  ARGO_HOME:  {} ({})", home.display(), home_status);
    println!("  profile:    {}", cfg.core.profile);
    println!(
        "  tenant:     {} (mode={})",
        cfg.tenant.default_id, cfg.tenant.mode
    );
    let bundle_dirs = ["skills", "personas", "cron", "rubrics"];
    let mut missing = Vec::new();
    for dir in bundle_dirs {
        if !cwd.join(dir).exists() {
            missing.push(dir);
        }
    }
    if missing.is_empty() {
        println!("  bundle dirs: all present");
    } else {
        println!(
            "  bundle dirs missing: {} (run `argo workspace init` to create)",
            missing.join(", ")
        );
    }
    let memfiles = ["MEMORY.md", "USER.md", "SOUL.md", "AGENTS.md"];
    let mut missing_m = Vec::new();
    for f in memfiles {
        if !cwd.join(f).exists() {
            missing_m.push(f);
        }
    }
    if missing_m.is_empty() {
        println!("  memory files: all present");
    } else {
        println!(
            "  memory files missing: {} (run `argo workspace init`)",
            missing_m.join(", ")
        );
    }
    Ok(())
}
