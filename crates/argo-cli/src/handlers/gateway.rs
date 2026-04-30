//! `argo gateway` — start / stop / status / list. The first three
//! need a real daemon binding and are deferred. \`status\` reads
//! \`argo.toml\` and reports the configured adapters so users can
//! verify their gateway config without spinning up the full
//! daemon.
use crate::cli::GatewayCmd;
use argo_config::ArgoConfig;
use argo_types::{ArgoError, ArgoResult};

pub fn run(cmd: GatewayCmd) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let cfg = ArgoConfig::load(&cwd.join("argo.toml")).unwrap_or_default();
    match cmd {
        GatewayCmd::Status => status(&cfg),
        other => {
            println!(
                "argo gateway: '{other:?}' requires the daemon's gateway runtime (separate PR)."
            );
            Ok(())
        }
    }
}

fn status(cfg: &ArgoConfig) -> ArgoResult<()> {
    println!("gateway:");
    println!(
        "  enabled:   {} (set [gateway].enabled in argo.toml)",
        cfg.gateway.enabled
    );
    if cfg.gateway.adapters.is_empty() {
        println!("  adapters:  (none configured)");
    } else {
        println!("  adapters:");
        for a in &cfg.gateway.adapters {
            println!("    - {a}");
        }
    }
    Ok(())
}
