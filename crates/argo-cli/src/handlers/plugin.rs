//! `argo plugin` — list / status. Install + run + the rest of the
//! lifecycle live with the daemon binding (separate PR).
use crate::cli::PluginCmd;
use argo_config::ArgoConfig;
use argo_types::{ArgoError, ArgoResult};

pub fn run(cmd: PluginCmd) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let cfg = ArgoConfig::load(&cwd.join("argo.toml")).unwrap_or_default();
    match cmd {
        PluginCmd::List { json, format } => list(&cfg, json, &format),
        other => {
            println!("argo plugin: '{other:?}' requires the daemon's plugin host (separate PR).");
            Ok(())
        }
    }
}

fn list(cfg: &ArgoConfig, json: bool, format: &str) -> ArgoResult<()> {
    let pick = if json { "json" } else { format };
    match pick {
        "json" => {
            let arr: Vec<&String> = cfg.plugin.auto_load.iter().collect();
            println!(
                "{}",
                serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string())
            );
            return Ok(());
        }
        "markdown" => {
            println!("# Plugins ({} configured)", cfg.plugin.auto_load.len());
            println!();
            if cfg.plugin.auto_load.is_empty() {
                println!("_(no plugins configured under [plugin].auto_load)_");
                return Ok(());
            }
            for name in &cfg.plugin.auto_load {
                println!("- `{name}`");
            }
            return Ok(());
        }
        "text" => {}
        other => {
            return Err(ArgoError::Provider(format!(
                "unknown format '{other}' (expected 'text', 'json', or 'markdown')"
            )));
        }
    }
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
