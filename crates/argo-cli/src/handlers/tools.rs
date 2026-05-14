//! `argo tools list / show <name>` — surface the in-core tool
//! registry. Mirrors the gateway's `GET /tools` and
//! `GET /tools/<name>` (#258) so the same registry is visible
//! offline (no daemon required).
use crate::cli::ToolsCmd;
use argo_tool::{builtin_tools, ToolRegistry};
use argo_types::{ArgoError, ArgoResult};

pub async fn run(cmd: Option<ToolsCmd>) -> ArgoResult<()> {
    match cmd {
        None => {
            list(false, "text", None).await
        }
        Some(ToolsCmd::List {
            json,
            format,
            name_contains,
        }) => list(json, &format, name_contains.as_deref()).await,
        Some(ToolsCmd::Show { name, json }) => show(&name, json),
    }
}

async fn list(json: bool, format: &str, name_contains: Option<&str>) -> ArgoResult<()> {
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let mut reg = ToolRegistry::new();
    for t in builtin_tools(cwd) {
        reg.register(t);
    }
    let needle = name_contains.map(str::to_lowercase);
    let mut rows: Vec<argo_tool::ToolSchema> = reg
        .schemas()
        .into_iter()
        .filter(|s| match needle.as_deref() {
            Some(n) => s.name.to_lowercase().contains(n),
            None => true,
        })
        .collect();
    rows.sort_by(|a, b| a.name.cmp(&b.name));
    let pick = if json { "json" } else { format };
    match pick {
        "json" => {
            for s in &rows {
                let row = serde_json::json!({
                    "name": s.name,
                    "description": s.description,
                    "parameters": s.parameters,
                });
                println!("{}", serde_json::to_string(&row).unwrap_or_default());
            }
        }
        "markdown" => {
            println!("# Tools ({} match{})", rows.len(), if rows.len() == 1 { "" } else { "es" });
            println!();
            if rows.is_empty() {
                return Ok(());
            }
            println!("| name | description |");
            println!("| --- | --- |");
            for s in &rows {
                let desc = s
                    .description
                    .lines()
                    .next()
                    .unwrap_or("")
                    .replace('|', "\\|");
                println!("| `{}` | {desc} |", s.name);
            }
        }
        "text" => {
            if rows.is_empty() {
                println!("(no tools)");
                return Ok(());
            }
            println!("{:<20} description", "name");
            for s in &rows {
                let head: String = s
                    .description
                    .lines()
                    .next()
                    .unwrap_or("")
                    .chars()
                    .take(70)
                    .collect();
                println!("{:<20} {head}", s.name);
            }
        }
        other => {
            return Err(ArgoError::Provider(format!(
                "unknown format '{other}' (expected 'text', 'json', or 'markdown')"
            )));
        }
    }
    Ok(())
}

fn show(name: &str, json: bool) -> ArgoResult<()> {
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let mut reg = ToolRegistry::new();
    for t in builtin_tools(cwd) {
        reg.register(t);
    }
    let s = reg
        .schemas()
        .into_iter()
        .find(|s| s.name == name)
        .ok_or_else(|| ArgoError::Provider(format!("no tool named '{name}'")))?;
    if json {
        let payload = serde_json::json!({
            "name": s.name,
            "description": s.description,
            "parameters": s.parameters,
        });
        println!("{}", serde_json::to_string(&payload).unwrap_or_default());
        return Ok(());
    }
    println!("# {}", s.name);
    println!();
    println!("{}", s.description);
    println!();
    println!("## Parameters");
    println!();
    println!(
        "{}",
        serde_json::to_string_pretty(&s.parameters).unwrap_or_default()
    );
    Ok(())
}
