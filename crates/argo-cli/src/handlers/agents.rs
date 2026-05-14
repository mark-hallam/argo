//! `argo agents list / create / delete` — surfaces the
//! workspace's `[agents.*]` blocks from `argo.toml` and lets
//! the user edit them inline. Live multi-agent state (when an
//! agent is "running" against a tenant gateway) needs the
//! daemon's gateway routing — that's a separate PR; the static
//! configuration covered here is pure file I/O.
use crate::cli::AgentsCmd;
use argo_types::{ArgoError, ArgoResult};

pub fn run(cmd: Option<AgentsCmd>) -> ArgoResult<()> {
    match cmd {
        None => list_agents(false, "text"),
        Some(AgentsCmd::List { json, format }) => list_agents(json, &format),
        Some(AgentsCmd::Create {
            name,
            model,
            toolset,
            memory_namespace,
        }) => create_agent(
            &name,
            model.as_deref(),
            toolset.as_deref(),
            memory_namespace.as_deref(),
        ),
        Some(AgentsCmd::Delete { name }) => delete_agent(&name),
    }
}

fn list_agents(json: bool, format: &str) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let toml_path = cwd.join("argo.toml");
    let pick = if json { "json" } else { format };
    if !toml_path.exists() {
        match pick {
            "json" => println!("[]"),
            "markdown" => println!("# Agents (0 matches)\n\n_(no argo.toml — run `argo workspace init` first)_"),
            _ => println!("(no argo.toml — run `argo workspace init` first)"),
        }
        return Ok(());
    }
    let body = std::fs::read_to_string(&toml_path)
        .map_err(|e| ArgoError::Io(format!("read {}: {e}", toml_path.display())))?;
    let value: toml::Value =
        toml::from_str(&body).map_err(|e| ArgoError::Provider(format!("parse argo.toml: {e}")))?;
    let Some(agents_table) = value.get("agents").and_then(|v| v.as_table()) else {
        match pick {
            "json" => println!("[]"),
            "markdown" => println!("# Agents (0 matches)\n\n_(no [agents.*] blocks in argo.toml)_"),
            _ => println!("(no [agents.*] blocks in argo.toml)"),
        }
        return Ok(());
    };
    if agents_table.is_empty() {
        match pick {
            "json" => println!("[]"),
            "markdown" => println!("# Agents (0 matches)\n\n_(no [agents.*] blocks in argo.toml)_"),
            _ => println!("(no [agents.*] blocks in argo.toml)"),
        }
        return Ok(());
    }
    let mut names: Vec<&String> = agents_table.keys().collect();
    names.sort();
    match pick {
        "json" => {
            let arr: Vec<serde_json::Value> = names
                .iter()
                .map(|name| {
                    let block = &agents_table[*name];
                    serde_json::json!({
                        "name": name,
                        "default_model": block.get("default_model").and_then(toml::Value::as_str),
                        "default_toolset": block.get("default_toolset").and_then(toml::Value::as_str),
                        "memory_namespace": block.get("memory_namespace").and_then(toml::Value::as_str),
                    })
                })
                .collect();
            println!(
                "{}",
                serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string())
            );
            return Ok(());
        }
        "markdown" => {
            println!("# Agents ({} matches)", names.len());
            println!();
            println!("| name | default_model | default_toolset | memory_namespace |");
            println!("| --- | --- | --- | --- |");
            for name in &names {
                let block = &agents_table[*name];
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
                println!("| `{name}` | `{model}` | `{toolset}` | `{ns}` |");
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
    println!(
        "{name:<16} {model:<28} {toolset:<14} memory_namespace",
        name = "name",
        model = "default_model",
        toolset = "default_toolset"
    );
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

fn create_agent(
    name: &str,
    model: Option<&str>,
    toolset: Option<&str>,
    memory_namespace: Option<&str>,
) -> ArgoResult<()> {
    if name.is_empty() {
        return Err(ArgoError::Provider("agents create: empty name".into()));
    }
    let (cwd, toml_path) = workspace_toml()?;
    let mut doc = parse_argo_toml(&toml_path)?;
    let agents = ensure_table(&mut doc, "agents")?;
    if agents.contains_key(name) {
        return Err(ArgoError::Provider(format!(
            "agent '{name}' already exists in {}",
            toml_path.display()
        )));
    }
    let mut block = toml::map::Map::new();
    block.insert(
        "default_model".into(),
        toml::Value::String(model.unwrap_or("claude-3-5-sonnet").to_string()),
    );
    block.insert(
        "default_toolset".into(),
        toml::Value::String(toolset.unwrap_or("default").to_string()),
    );
    block.insert(
        "memory_namespace".into(),
        toml::Value::String(memory_namespace.unwrap_or(name).to_string()),
    );
    agents.insert(name.into(), toml::Value::Table(block));
    write_argo_toml(&toml_path, &doc)?;
    println!(
        "argo agents create: added [agents.{name}] to {}/argo.toml",
        cwd.display()
    );
    Ok(())
}

fn delete_agent(name: &str) -> ArgoResult<()> {
    let (cwd, toml_path) = workspace_toml()?;
    let mut doc = parse_argo_toml(&toml_path)?;
    let agents = ensure_table(&mut doc, "agents")?;
    if agents.remove(name).is_none() {
        return Err(ArgoError::Provider(format!(
            "agent '{name}' not found in {}",
            toml_path.display()
        )));
    }
    write_argo_toml(&toml_path, &doc)?;
    println!(
        "argo agents delete: removed [agents.{name}] from {}/argo.toml",
        cwd.display()
    );
    Ok(())
}

fn workspace_toml() -> ArgoResult<(std::path::PathBuf, std::path::PathBuf)> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let toml_path = cwd.join("argo.toml");
    if !toml_path.exists() {
        return Err(ArgoError::Provider(format!(
            "no argo.toml at {} — run `argo workspace init` first",
            toml_path.display()
        )));
    }
    Ok((cwd, toml_path))
}

fn parse_argo_toml(toml_path: &std::path::Path) -> ArgoResult<toml::Value> {
    let body = std::fs::read_to_string(toml_path)
        .map_err(|e| ArgoError::Io(format!("read {}: {e}", toml_path.display())))?;
    toml::from_str(&body).map_err(|e| ArgoError::Provider(format!("parse argo.toml: {e}")))
}

fn ensure_table<'a>(
    doc: &'a mut toml::Value,
    key: &str,
) -> ArgoResult<&'a mut toml::map::Map<String, toml::Value>> {
    let table = doc
        .as_table_mut()
        .ok_or_else(|| ArgoError::Provider("argo.toml: top-level is not a table".into()))?;
    let entry = table
        .entry(key.to_string())
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
    entry
        .as_table_mut()
        .ok_or_else(|| ArgoError::Provider(format!("argo.toml: [{key}] is not a table")))
}

fn write_argo_toml(toml_path: &std::path::Path, doc: &toml::Value) -> ArgoResult<()> {
    // Note: comments in argo.toml are dropped on rewrite — `toml`
    // (the crate) doesn't preserve them. Acceptable for v1; a
    // future PR can swap in `toml_edit` if comment preservation
    // becomes a real ask.
    let serialized = toml::to_string_pretty(doc)
        .map_err(|e| ArgoError::Provider(format!("serialize argo.toml: {e}")))?;
    std::fs::write(toml_path, serialized)
        .map_err(|e| ArgoError::Io(format!("write {}: {e}", toml_path.display())))
}
