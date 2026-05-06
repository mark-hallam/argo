use super::vertical::resolve_verticals_dir;
use argo_types::{ArgoError, ArgoResult};
use argo_vertical::{load_dir, ActionRiskLevel, LoadedVertical};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize)]
struct VerticalDetails {
    id: String,
    name: String,
    version: String,
    description: String,
    root: String,
    persona: Option<PersonaDetails>,
    action_count: usize,
    actions: Vec<ActionDetails>,
    eval_seed_count: usize,
    skills_bundled: Vec<String>,
    memory_entries: Option<Vec<MemoryEntryDetails>>,
    vertical_toml: String,
}

#[derive(Debug, Serialize)]
struct PersonaDetails {
    path: String,
    description: String,
}

#[derive(Debug, Serialize)]
struct ActionDetails {
    name: String,
    risk: &'static str,
    requires_approval: bool,
    description: String,
}

#[derive(Debug, Serialize)]
struct MemoryEntryDetails {
    key: String,
    #[serde(rename = "type")]
    type_: String,
    purpose: String,
}

#[derive(Debug, Serialize)]
struct DescribeOutput {
    id: String,
    description: String,
}

pub async fn show(id: &str, json: bool) -> ArgoResult<()> {
    let loaded = load_vertical(id)?;
    let details = details_for(loaded)?;
    if json {
        print_json(&details);
    } else {
        print_show(&details);
    }
    Ok(())
}

pub async fn describe(id: &str, json: bool) -> ArgoResult<()> {
    let loaded = load_vertical(id)?;
    let details = details_for(loaded)?;
    let description = describe_details(&details);
    if json {
        print_json(&DescribeOutput {
            id: details.id,
            description,
        });
    } else {
        println!("{description}");
    }
    Ok(())
}

fn load_vertical(id: &str) -> ArgoResult<LoadedVertical> {
    let dir = resolve_verticals_dir();
    let (registry, _errors) = load_dir(&dir)?;
    registry.get(id).cloned().ok_or_else(|| {
        ArgoError::Provider(format!("vertical '{id}' not found in {}", dir.display()))
    })
}

fn details_for(v: LoadedVertical) -> ArgoResult<VerticalDetails> {
    let m = &v.manifest;
    let actions = m
        .actions
        .as_ref()
        .map(|section| {
            section
                .actions
                .iter()
                .map(|(name, decl)| ActionDetails {
                    name: name.clone(),
                    risk: risk_label(decl.risk),
                    requires_approval: decl.requires_approval,
                    description: decl.description.clone(),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Ok(VerticalDetails {
        id: m.vertical.id.clone(),
        name: m.vertical.name.clone(),
        version: m.vertical.version.clone(),
        description: m.vertical.description.clone(),
        root: v.root.display().to_string(),
        persona: persona_details(&v)?,
        action_count: actions.len(),
        actions,
        eval_seed_count: eval_seed_count(&v)?,
        skills_bundled: bundled_skills(&v)?,
        memory_entries: memory_entries(&v)?,
        vertical_toml: read_file(&v.root.join("vertical.toml"))?,
    })
}

fn persona_details(v: &LoadedVertical) -> ArgoResult<Option<PersonaDetails>> {
    let Some(role) = &v.manifest.role else {
        return Ok(None);
    };
    let Some(path) = &role.persona else {
        return Ok(None);
    };
    Ok(Some(PersonaDetails {
        path: path.clone(),
        description: read_file(&v.root.join(path))?.trim().to_string(),
    }))
}

fn eval_seed_count(v: &LoadedVertical) -> ArgoResult<usize> {
    let Some(eval) = &v.manifest.evaluation else {
        return Ok(0);
    };
    let mut total = 0;
    for rel in &eval.suites {
        let body = read_file(&v.root.join(rel))?;
        total += body.lines().filter(|l| !l.trim().is_empty()).count();
    }
    Ok(total)
}

fn bundled_skills(v: &LoadedVertical) -> ArgoResult<Vec<String>> {
    let Some(skills) = &v.manifest.skills else {
        return Ok(Vec::new());
    };
    let dir = v.root.join(&skills.bundle);
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&dir)
        .map_err(|e| ArgoError::Io(format!("read_dir {}: {e}", dir.display())))?
    {
        let entry = entry.map_err(|e| ArgoError::Io(format!("read_dir {}: {e}", dir.display())))?;
        let path = entry.path();
        if path.is_file() {
            out.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    out.sort();
    Ok(out)
}

fn memory_entries(v: &LoadedVertical) -> ArgoResult<Option<Vec<MemoryEntryDetails>>> {
    let Some(memory) = &v.manifest.memory else {
        return Ok(None);
    };
    let Some(schema) = &memory.schema else {
        return Ok(None);
    };
    let body = read_file(&v.root.join(schema))?;
    let schema = argo_vertical::parse_memory_schema(&body)
        .map_err(|e| ArgoError::Schema(format!("[memory].schema: {e}")))?;
    Ok(Some(
        schema
            .entries
            .into_iter()
            .map(|e| MemoryEntryDetails {
                key: e.key,
                type_: e.type_,
                purpose: e.purpose,
            })
            .collect(),
    ))
}

fn read_file(path: &Path) -> ArgoResult<String> {
    std::fs::read_to_string(path)
        .map_err(|e| ArgoError::Io(format!("read {}: {e}", path.display())))
}

fn print_show(d: &VerticalDetails) {
    println!("vertical {}: {} v{}", d.id, d.name, d.version);
    println!("  root: {}", d.root);
    if !d.description.is_empty() {
        println!("  description: {}", d.description);
    }
    match &d.persona {
        Some(persona) => {
            println!("  persona: {}", persona.path);
            println!("  persona description: {}", one_line(&persona.description));
        }
        None => println!("  persona: (none)"),
    }
    println!("  action count: {}", d.action_count);
    if d.actions.is_empty() {
        println!("  actions: (none)");
    } else {
        println!("  actions:");
        for action in &d.actions {
            println!(
                "    {:<24} risk={:<6} approval={:<5} {}",
                action.name, action.risk, action.requires_approval, action.description
            );
        }
    }
    println!("  eval seed count: {}", d.eval_seed_count);
    if d.skills_bundled.is_empty() {
        println!("  skills bundled: (none)");
    } else {
        println!("  skills bundled: {}", d.skills_bundled.join(", "));
    }
    println!("vertical.toml:");
    for line in d.vertical_toml.lines() {
        println!("  {line}");
    }
}

fn describe_details(d: &VerticalDetails) -> String {
    let approval_count = d.actions.iter().filter(|a| a.requires_approval).count();
    let persona = d
        .persona
        .as_ref()
        .map(|p| p.path.as_str())
        .unwrap_or("no persona file");
    let mut text = format!(
        "{} (`{}`) v{} is a workspace vertical with {} declared action{}, {} approval-gated, {}, {} eval seed{}, and {} bundled skill{}.",
        d.name,
        d.id,
        d.version,
        d.action_count,
        plural(d.action_count),
        approval_count,
        persona,
        d.eval_seed_count,
        plural(d.eval_seed_count),
        d.skills_bundled.len(),
        plural(d.skills_bundled.len())
    );
    if !d.description.is_empty() {
        text.push(' ');
        text.push_str(&one_line(&d.description));
    }
    limit_chars(text, 300)
}

fn one_line(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn limit_chars(text: String, max: usize) -> String {
    if text.chars().count() <= max {
        return text;
    }
    let mut out = text.chars().take(max.saturating_sub(3)).collect::<String>();
    out.push_str("...");
    out
}

fn plural(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

fn risk_label(risk: ActionRiskLevel) -> &'static str {
    match risk {
        ActionRiskLevel::Low => "low",
        ActionRiskLevel::Medium => "medium",
        ActionRiskLevel::High => "high",
    }
}

fn print_json<T: Serialize>(value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(body) => println!("{body}"),
        Err(e) => println!(r#"{{"error":"json encode: {e}"}}"#),
    }
}
