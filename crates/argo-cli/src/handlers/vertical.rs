//! `argo vertical …` — read-only discovery surface for the
//! vertical-template system (#650).
//!
//! Backed by [`argo_vertical`] for parsing/validation; this module
//! only adds policy (where to discover verticals) and rendering
//! (text / JSON / markdown). The single source of truth for what
//! a vertical *is* lives in `argo-vertical`.
//!
//! Out of scope for this PR per the agreed phased plan: install,
//! eval, run, session binding, gateway dynamic mounts.
use crate::cli::VerticalCmd;
use argo_types::{ArgoError, ArgoResult};
use argo_vertical::{load_dir, LoadedVertical};
use std::path::PathBuf;

/// Resolve where the loader walks. Delegates to
/// `argo_vertical::default_dir()` so CLI + gateway + agent
/// runner all agree on where to look (#654).
pub(crate) fn resolve_verticals_dir() -> PathBuf {
    argo_vertical::default_dir()
}

pub async fn run(cmd: VerticalCmd) -> ArgoResult<()> {
    match cmd {
        VerticalCmd::List { json } => list(json).await,
        VerticalCmd::Show { id, json, format } => show(&id, json, &format).await,
        VerticalCmd::Validate { id, json } => validate(&id, json).await,
        VerticalCmd::Eval { id, json } => super::vertical_eval::run(&id, json).await,
        VerticalCmd::Workflow {
            vertical_id,
            workflow_name,
            session,
            json,
        } => {
            super::vertical_workflow::run(
                &vertical_id,
                &workflow_name,
                session.as_deref(),
                json,
            )
            .await
        }
        VerticalCmd::Install {
            source,
            target,
            force,
            json,
        } => super::vertical_install::install(&source, target.as_deref(), force, json).await,
        VerticalCmd::Uninstall {
            id,
            target,
            yes,
            json,
        } => super::vertical_install::uninstall(&id, target.as_deref(), yes, json).await,
        VerticalCmd::Run {
            vertical_id,
            workflow,
            title,
            provider,
            model,
            json,
        } => {
            super::vertical_run::run(
                &vertical_id,
                &workflow,
                title.as_deref(),
                &provider,
                &model,
                json,
            )
            .await
        }
    }
}

async fn list(json: bool) -> ArgoResult<()> {
    let dir = resolve_verticals_dir();
    let (registry, errors) = load_dir(&dir)?;
    if json {
        let arr: Vec<serde_json::Value> = registry
            .list()
            .map(|(id, v)| {
                serde_json::json!({
                    "id": id,
                    "name": v.manifest.vertical.name,
                    "version": v.manifest.vertical.version,
                    "description": v.manifest.vertical.description,
                    "tags": v.manifest.vertical.tags,
                    "root": v.root.display().to_string(),
                })
            })
            .collect();
        let errs: Vec<serde_json::Value> = errors
            .iter()
            .map(|e| {
                serde_json::json!({
                    "dir": e.dir.display().to_string(),
                    "error": format!("{}", e.error),
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "ok": errors.is_empty(),
                "loaded": arr,
                "errors": errs,
            }))
            .unwrap_or_default()
        );
        return Ok(());
    }
    if registry.is_empty() && errors.is_empty() {
        println!("(no verticals in {})", dir.display());
        return Ok(());
    }
    println!("verticals in {}:", dir.display());
    for (id, v) in registry.list() {
        println!(
            "  {id:<20} {} v{}",
            v.manifest.vertical.name, v.manifest.vertical.version
        );
    }
    if !errors.is_empty() {
        println!();
        println!("errors:");
        for e in &errors {
            println!("  {}: {}", e.dir.display(), e.error);
        }
    }
    Ok(())
}

async fn show(id: &str, json: bool, format: &str) -> ArgoResult<()> {
    let dir = resolve_verticals_dir();
    let (registry, _errors) = load_dir(&dir)?;
    let loaded = registry.get(id).ok_or_else(|| {
        ArgoError::Provider(format!(
            "vertical '{id}' not found in {}",
            dir.display()
        ))
    })?;
    if json {
        println!(
            "{}",
            serde_json::to_string(&summarize_loaded(loaded)).unwrap_or_default()
        );
        return Ok(());
    }
    if format == "markdown" {
        print!("{}", render_markdown(loaded));
        return Ok(());
    }
    let m = &loaded.manifest;
    println!(
        "vertical {}: {} v{}",
        m.vertical.id, m.vertical.name, m.vertical.version
    );
    if !m.vertical.description.is_empty() {
        println!("  description: {}", m.vertical.description);
    }
    println!("  root: {}", loaded.root.display());
    if let Some(role) = &m.role {
        println!("  system_prompt: {}", role.system_prompt);
    }
    if let Some(tools) = &m.tools {
        println!(
            "  tools: {} allowed, {} denied",
            tools.allowed.len(),
            tools.denied.len()
        );
    }
    if let Some(eval) = &m.evaluation {
        println!("  eval suites: {}", eval.suites.len());
    }
    if let Some(gw) = &m.gateway {
        println!("  gateway endpoints: {}", gw.public_endpoints.len());
    }
    // Memory typed-entries summary (#663). Read-only,
    // non-fatal: a malformed schema would have already failed
    // validate; here we just surface what the agent + operator
    // are working with.
    if let Some(mem) = &m.memory {
        if let Some(schema_rel) = &mem.schema {
            let p = loaded.root.join(schema_rel);
            if let Ok(src) = std::fs::read_to_string(&p) {
                if let Ok(s) = argo_vertical::parse_memory_schema(&src) {
                    println!("  memory entries: {}", s.entries.len());
                }
            }
        }
    }
    Ok(())
}

async fn validate(id: &str, json: bool) -> ArgoResult<()> {
    let dir = resolve_verticals_dir();
    let (registry, errors) = load_dir(&dir)?;

    // The loader already collected per-vertical errors. If `id`
    // matches one of them, surface that as the validation failure.
    if let Some(load_err) = errors
        .iter()
        .find(|e| e.dir.file_name().and_then(|n| n.to_str()) == Some(id))
    {
        let msg = format!("{}", load_err.error);
        if json {
            println!(
                "{}",
                serde_json::to_string(&serde_json::json!({
                    "id": id,
                    "valid": false,
                    "error": msg,
                }))
                .unwrap_or_default()
            );
        } else {
            println!("vertical '{id}': INVALID");
            println!("  {msg}");
        }
        return Err(ArgoError::Schema(msg));
    }

    let loaded = registry.get(id).ok_or_else(|| {
        ArgoError::Provider(format!(
            "vertical '{id}' not found in {}",
            dir.display()
        ))
    })?;

    // Re-run validate explicitly so the operator sees a clean
    // pass even when nothing changed since loader-time.
    argo_vertical::validate(&loaded.manifest, &loaded.root)?;

    if json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "id": id,
                "valid": true,
            }))
            .unwrap_or_default()
        );
    } else {
        println!("vertical '{id}': OK");
    }
    Ok(())
}

/// JSON projection of a loaded vertical. Shared by the CLI's
/// `show --json` and the gateway's `GET /verticals/<id>` so the
/// wire shape stays in lock-step.
pub(crate) fn summarize_loaded(v: &LoadedVertical) -> serde_json::Value {
    let m = &v.manifest;
    let memory_entries = m.memory.as_ref().and_then(|mem| {
        let rel = mem.schema.as_ref()?;
        let body = std::fs::read_to_string(v.root.join(rel)).ok()?;
        let s = argo_vertical::parse_memory_schema(&body).ok()?;
        Some(
            s.entries
                .into_iter()
                .map(|e| {
                    serde_json::json!({
                        "key": e.key,
                        "type": e.type_,
                        "purpose": e.purpose,
                    })
                })
                .collect::<Vec<_>>(),
        )
    });
    serde_json::json!({
        "id": m.vertical.id,
        "name": m.vertical.name,
        "version": m.vertical.version,
        "description": m.vertical.description,
        "tags": m.vertical.tags,
        "authors": m.vertical.authors,
        "root": v.root.display().to_string(),
        "role": m.role.as_ref().map(|r| serde_json::json!({
            "system_prompt": r.system_prompt,
            "persona": r.persona,
        })),
        "provider": m.provider.as_ref().map(|p| serde_json::json!({
            "default": p.default,
            "fallback": p.fallback,
            "cost_tier": p.cost_tier,
        })),
        "tools": m.tools.as_ref().map(|t| serde_json::json!({
            "allowed": t.allowed,
            "denied": t.denied,
        })),
        "skills_bundle": m.skills.as_ref().map(|s| s.bundle.clone()),
        "eval_suites": m.evaluation.as_ref()
            .map(|e| e.suites.clone()).unwrap_or_default(),
        "gateway_endpoints": m.gateway.as_ref()
            .map(|g| g.public_endpoints.clone()).unwrap_or_default(),
        "memory_entries": memory_entries,
    })
}

/// Markdown projection. Shared by CLI's `show --format markdown`
/// and the gateway's `?format=markdown`.
pub(crate) fn render_markdown(v: &LoadedVertical) -> String {
    let m = &v.manifest;
    let mut out = String::new();
    out.push_str(&format!(
        "# {} (`{}`)\n\n",
        m.vertical.name, m.vertical.id
    ));
    if !m.vertical.description.is_empty() {
        out.push_str(&format!("{}\n\n", m.vertical.description));
    }
    out.push_str(&format!("- **version**: {}\n", m.vertical.version));
    out.push_str(&format!("- **root**: `{}`\n", v.root.display()));
    if !m.vertical.tags.is_empty() {
        out.push_str(&format!(
            "- **tags**: {}\n",
            m.vertical.tags.join(", ")
        ));
    }
    if let Some(role) = &m.role {
        out.push_str("\n## Role\n\n");
        out.push_str(&format!("- system_prompt: `{}`\n", role.system_prompt));
        if let Some(persona) = &role.persona {
            out.push_str(&format!("- persona: `{persona}`\n"));
        }
    }
    if let Some(prov) = &m.provider {
        out.push_str("\n## Provider\n\n");
        out.push_str(&format!("- default: `{}`\n", prov.default));
        if let Some(fb) = &prov.fallback {
            out.push_str(&format!("- fallback: `{fb}`\n"));
        }
        out.push_str(&format!("- cost_tier: `{}`\n", prov.cost_tier));
    }
    if let Some(tools) = &m.tools {
        out.push_str("\n## Tools\n\n");
        out.push_str(&format!(
            "- allowed ({}): `{}`\n",
            tools.allowed.len(),
            tools.allowed.join(", ")
        ));
        out.push_str(&format!(
            "- denied ({}): `{}`\n",
            tools.denied.len(),
            tools.denied.join(", ")
        ));
    }
    if let Some(eval) = &m.evaluation {
        out.push_str(&format!(
            "\n## Evaluation\n\n- {} suite(s)\n",
            eval.suites.len()
        ));
    }
    if let Some(gw) = &m.gateway {
        out.push_str(&format!(
            "\n## Gateway\n\n- {} public endpoint(s)\n",
            gw.public_endpoints.len()
        ));
    }
    out
}
