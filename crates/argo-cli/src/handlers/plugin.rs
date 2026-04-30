//! `argo plugin` — list / status. Install + run + the rest of the
//! lifecycle live with the daemon binding (separate PR).
use crate::cli::PluginCmd;
use argo_config::ArgoConfig;
use argo_types::{ArgoError, ArgoResult};

pub fn run(cmd: PluginCmd) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let cfg = ArgoConfig::load(&cwd.join("argo.toml")).unwrap_or_default();
    match cmd {
        PluginCmd::List => list(&cfg),
        other => {
            println!("argo plugin: '{other:?}' requires the daemon's plugin host (separate PR).");
            Ok(())
        }
    }
}

fn list(cfg: &ArgoConfig) -> ArgoResult<()> {
    if cfg.plugin.auto_load.is_empty() {
        println!("(no plugins configured under [plugin].auto_load)");
        return Ok(());
    }
    println!("auto_load:");
    for name in &cfg.plugin.auto_load {
        println!("  {name}");
    }
    Ok(())
}
