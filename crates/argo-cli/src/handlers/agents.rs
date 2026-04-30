//! `argo agents list` — surfaces the workspace's `[agents.*]`
//! blocks from \`argo.toml\` so users can see which named agents
//! are configured (Amendment 5: native multi-agent per tenant).
//!
//! Create / delete need persistence beyond a flat-file edit
//! (the daemon manages live state); they fall through with a
//! daemon-binding message until the gateway PR lands.
use crate::cli::AgentsCmd;
use argo_types::{ArgoError, ArgoResult};

pub fn run(cmd: Option<AgentsCmd>) -> ArgoResult<()> {
    match cmd {
        None | Some(AgentsCmd::List) => list_agents(),
        Some(other) => {
            println!(
                "argo agents: '{other:?}' requires the daemon's gateway routing (separate PR)."
            );
            Ok(())
        }
    }
}

fn list_agents() -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let toml_path = cwd.join("argo.toml");
    if !toml_path.exists() {
        println!("(no argo.toml — run `argo workspace init` first)");
        return Ok(());
    }
    let body = std::fs::read_to_string(&toml_path)
        .map_err(|e| ArgoError::Provider(format!("read {}: {e}", toml_path.display())))?;
    let value: toml::Value =
        toml::from_str(&body).map_err(|e| ArgoError::Provider(format!("parse argo.toml: {e}")))?;
    let Some(agents_table) = value.get("agents").and_then(|v| v.as_table()) else {
        println!("(no [agents.*] blocks in argo.toml)");
        return Ok(());
    };
    if agents_table.is_empty() {
        println!("(no [agents.*] blocks in argo.toml)");
        return Ok(());
    }
    println!(
        "{name:<16} {model:<28} {toolset:<14} memory_namespace",
        name = "name",
        model = "default_model",
        toolset = "default_toolset"
    );
    let mut names: Vec<&String> = agents_table.keys().collect();
    names.sort();
    for name in names {
        let block = &agents_table[name];
        let model = block
            .get("default_model")
            .and_then(toml::Value::as_str)
            .unwrap_or("(none)");
        let toolset = block
            .get("default_toolset")
            .and_then(toml::Value::as_str)
            .unwrap_or("(none)");
        let ns = block
            .get("memory_namespace")
            .and_then(toml::Value::as_str)
            .unwrap_or("(none)");
        println!("{name:<16} {model:<28} {toolset:<14} {ns}");
    }
    Ok(())
}
