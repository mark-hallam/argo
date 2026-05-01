//! `argo workspace` — bundle init / status / verify. The
//! marketplace-flavoured subcommands (push / pull / fork / diff
//! / artifacts) live in `workspace_marketplace.rs` so this file
//! stays under the 500-LOC ceiling.
use crate::cli::WorkspaceCmd;
use crate::handlers::workspace_marketplace as wm;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_config::ArgoConfig;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};

const DEFAULT_ARGO_TOML: &str = r#"# argo.toml — see `docs/plan/argo-build-plan.md` §3.
[core]
home = "~/.argo"
workspace = "."
profile = "default"

[tenant]
mode = "single"
default_id = "default"

[budget]
default_iterations = 60
default_spend_per_session_usd_micros = 1_000_000
default_spend_per_day_usd_micros    = 10_000_000
default_spend_per_month_usd_micros  = 100_000_000
notify_at_pct = [80, 95]
on_max_iterations = "prompt_user"

[loop]
on_restart = "prompt_user"

[routing]
policy = "fixed"

[judge]
enabled = true
rubric = "default"

[forge]
enabled = false
"#;

pub async fn run(cmd: WorkspaceCmd) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    match cmd {
        WorkspaceCmd::Init => init(&cwd).await,
        WorkspaceCmd::Status { json, format } => status(&cwd, json, &format).await,
        WorkspaceCmd::Verify => verify(&cwd),
        WorkspaceCmd::Push { oci_ref } => wm::push(&cwd, &oci_ref).await,
        WorkspaceCmd::Pull { oci_ref } => wm::pull(&cwd, &oci_ref).await,
        WorkspaceCmd::Artifacts { limit, json } => wm::artifacts(&cwd, limit, json).await,
        WorkspaceCmd::Diff { other } => wm::diff(&cwd, &other).await,
        WorkspaceCmd::Fork { source, into } => wm::fork(&cwd, &source, into.as_deref()).await,
        WorkspaceCmd::Publish => {
            println!("workspace publish: lands with the §23.3 release-pipeline wiring.");
            Ok(())
        }
    }
}

async fn status(cwd: &std::path::Path, json: bool, format: &str) -> ArgoResult<()> {
    let cfg = ArgoConfig::load(&cwd.join("argo.toml")).unwrap_or_default();
    let home = resolve_home(cwd)?;
    ensure_dir(&home)?;
    let db = db_path(&home);
    let store = SessionStore::open(&db).await?;
    store.migrate().await?;
    let stats = store.db_stats().await?;
    let ceilings = cfg.budget.to_cost_ceilings();
    let pick = if json { "json" } else { format };
    match pick {
        "json" => {
            let payload = serde_json::json!({
                "cwd":         cwd.display().to_string(),
                "argo_home":   home.display().to_string(),
                "profile":     cfg.core.profile,
                "tenant":      {
                    "default_id": cfg.tenant.default_id,
                    "mode":       cfg.tenant.mode,
                },
                "caps": {
                    "per_session_usd_micros": ceilings.per_session_usd_micros,
                    "per_day_usd_micros":     ceilings.per_day_usd_micros,
                    "per_month_usd_micros":   ceilings.per_month_usd_micros,
                },
                "rows": {
                    "sessions":   stats.sessions,
                    "messages":   stats.messages,
                    "audit":      stats.audit_rows,
                    "forge_runs": stats.forge_runs,
                },
            });
            println!("{}", serde_json::to_string(&payload).unwrap_or_default());
            return Ok(());
        }
        "markdown" => {
            println!("# Workspace status");
            println!();
            println!("- **cwd**: `{}`", cwd.display());
            println!("- **ARGO_HOME**: `{}`", home.display());
            println!("- **profile**: `{}`", cfg.core.profile);
            println!(
                "- **tenant**: `{}` (mode=`{}`)",
                cfg.tenant.default_id, cfg.tenant.mode
            );
            println!();
            println!("## Caps (USD)");
            println!();
            println!("- **per-session**: {}", format_cap(ceilings.per_session_usd_micros));
            println!("- **per-day**: {}", format_cap(ceilings.per_day_usd_micros));
            println!("- **per-month**: {}", format_cap(ceilings.per_month_usd_micros));
            println!();
            println!("## Rows");
            println!();
            println!("- **sessions**: {}", stats.sessions);
            println!("- **messages**: {}", stats.messages);
            println!("- **audit**: {}", stats.audit_rows);
            println!("- **forge runs**: {}", stats.forge_runs);
            return Ok(());
        }
        "text" => {}
        other => {
            return Err(ArgoError::Provider(format!(
                "unknown format '{other}' (expected 'text', 'json', or 'markdown')"
            )));
        }
    }
    println!("workspace status");
    println!("  cwd:        {}", cwd.display());
    println!("  ARGO_HOME:  {}", home.display());
    println!("  profile:    {}", cfg.core.profile);
    println!(
        "  tenant:     {} (mode={})",
        cfg.tenant.default_id, cfg.tenant.mode
    );
    println!(
        "  caps:       session={}  day={}  month={}",
        format_cap(ceilings.per_session_usd_micros),
        format_cap(ceilings.per_day_usd_micros),
        format_cap(ceilings.per_month_usd_micros),
    );
    println!(
        "  rows:       sessions={}  messages={}  audit={}  forge_runs={}",
        stats.sessions, stats.messages, stats.audit_rows, stats.forge_runs
    );
    Ok(())
}

fn verify(cwd: &std::path::Path) -> ArgoResult<()> {
    let mut problems: Vec<String> = Vec::new();
    // 1. argo.toml exists + parses
    let toml_path = cwd.join("argo.toml");
    if !toml_path.exists() {
        problems.push(format!("missing argo.toml at {}", toml_path.display()));
    }
    let body = std::fs::read_to_string(&toml_path).unwrap_or_default();
    let value: toml::Value = match toml::from_str(&body) {
        Ok(v) => v,
        Err(e) => {
            problems.push(format!("argo.toml parse error: {e}"));
            toml::Value::Table(toml::map::Map::new())
        }
    };
    // 2. memory files
    for f in ["MEMORY.md", "USER.md", "SOUL.md", "AGENTS.md"] {
        if !cwd.join(f).exists() {
            problems.push(format!("missing memory file: {f}"));
        }
    }
    // 3. bundle dirs
    for d in ["skills", "personas", "cron", "rubrics"] {
        if !cwd.join(d).exists() {
            problems.push(format!("missing bundle dir: {d}/"));
        }
    }
    // 4. agent persona files exist (per [agents.*].persona_file)
    if let Some(agents) = value.get("agents").and_then(toml::Value::as_table) {
        for (name, block) in agents {
            if let Some(persona_rel) = block.get("persona_file").and_then(toml::Value::as_str) {
                let persona_path = cwd.join(persona_rel);
                if !persona_path.exists() {
                    problems.push(format!(
                        "agent '{name}' persona_file missing: {}",
                        persona_path.display()
                    ));
                }
            }
        }
    }
    // 5. provider profiles resolve to bundled manifests
    if let Some(provider) = value.get("provider").and_then(toml::Value::as_table) {
        let manifests_dir = cwd.join("manifests").join("providers");
        let known_ids = if manifests_dir.exists() {
            collect_manifest_ids(&manifests_dir)
        } else {
            Vec::new()
        };
        for (profile_name, block) in provider {
            if profile_name == "default" || profile_name == "profiles" {
                continue;
            }
            if let Some(id) = block.get("id").and_then(toml::Value::as_str) {
                if !known_ids.iter().any(|k| k == id) {
                    problems.push(format!(
                        "[provider.{profile_name}].id = '{id}' has no matching manifest"
                    ));
                }
            }
        }
    }
    if problems.is_empty() {
        println!("workspace ok: {}", cwd.display());
        Ok(())
    } else {
        for p in &problems {
            eprintln!("  - {p}");
        }
        Err(ArgoError::Provider(format!(
            "{} integrity issue(s) in {}",
            problems.len(),
            cwd.display()
        )))
    }
}

fn collect_manifest_ids(dir: &std::path::Path) -> Vec<String> {
    let mut ids = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return ids;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().is_none_or(|x| x != "toml") {
            continue;
        }
        if let Ok(body) = std::fs::read_to_string(&p) {
            if let Ok(m) = argo_provider::ProviderManifest::from_toml_str(&body) {
                ids.push(m.provider.id);
            }
        }
    }
    ids
}

fn format_cap(value: Option<u64>) -> String {
    match value {
        Some(v) => format!("${:.2}", v as f64 / 1_000_000.0),
        None => "(unlimited)".into(),
    }
}

async fn init(cwd: &std::path::Path) -> ArgoResult<()> {
    let toml_path = cwd.join("argo.toml");
    if toml_path.exists() {
        return Err(ArgoError::Provider(format!(
            "{} already exists; refusing to overwrite",
            toml_path.display()
        )));
    }
    std::fs::write(&toml_path, DEFAULT_ARGO_TOML)
        .map_err(|e| ArgoError::Provider(format!("write argo.toml: {e}")))?;
    for dir in ["skills", "personas", "cron", "rubrics"] {
        let p = cwd.join(dir);
        std::fs::create_dir_all(&p)
            .map_err(|e| ArgoError::Provider(format!("mkdir {}: {e}", p.display())))?;
    }
    for memfile in ["MEMORY.md", "USER.md", "SOUL.md", "AGENTS.md"] {
        let p = cwd.join(memfile);
        if !p.exists() {
            std::fs::write(&p, format!("# {memfile}\n"))
                .map_err(|e| ArgoError::Provider(format!("write {}: {e}", p.display())))?;
        }
    }
    let home = resolve_home(cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (_tenant, workspace) = store.ensure_workspace(name, &root).await?;
    println!("workspace initialized at {}", cwd.display());
    println!("  + argo.toml");
    println!("  + skills/, personas/, cron/, rubrics/");
    println!("  + MEMORY.md, USER.md, SOUL.md, AGENTS.md");
    println!("  + db row: workspace {workspace}");
    Ok(())
}

