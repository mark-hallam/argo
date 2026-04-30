//! `argo workspace` — bundle init / push / pull / diff.
use crate::cli::WorkspaceCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_config::ArgoConfig;
use argo_marketplace::{
    build_bundle, OciDistributionRegistry, OciReference, OciRegistry,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use std::path::PathBuf;

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
        WorkspaceCmd::Status => status(&cwd).await,
        WorkspaceCmd::Verify => verify(&cwd),
        WorkspaceCmd::Push { oci_ref } => push(&cwd, &oci_ref).await,
        WorkspaceCmd::Pull { oci_ref } => pull(&cwd, &oci_ref).await,
        WorkspaceCmd::Diff { other } => {
            println!("workspace diff vs {other}: 3-way merge UI lands with §23.3 wiring.");
            Ok(())
        }
        WorkspaceCmd::Fork { source } => {
            println!("workspace fork from {source}: requires the OCI pull half (§23.3 follow-up).");
            Ok(())
        }
        WorkspaceCmd::Publish => {
            println!("workspace publish: lands with the §23.3 release-pipeline wiring.");
            Ok(())
        }
    }
}

async fn status(cwd: &std::path::Path) -> ArgoResult<()> {
    let cfg = ArgoConfig::load(&cwd.join("argo.toml")).unwrap_or_default();
    let home = resolve_home(cwd)?;
    ensure_dir(&home)?;
    let db = db_path(&home);
    let store = SessionStore::open(&db).await?;
    store.migrate().await?;
    let stats = store.db_stats().await?;
    let ceilings = cfg.budget.to_cost_ceilings();
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

async fn push(cwd: &std::path::Path, oci_ref: &str) -> ArgoResult<()> {
    let reference = OciReference::parse(oci_ref)?;
    let out_tar: PathBuf = std::env::temp_dir().join(format!(
        "argo-workspace-{}-{}.tar",
        reference.repo.replace('/', "-"),
        reference.tag
    ));
    let manifest = build_bundle(cwd, &reference.repo, &reference.tag, &out_tar).await?;
    println!("built bundle: {}", out_tar.display());
    println!("  workspace: {}", manifest.workspace_name);
    println!("  tag:       {}", manifest.tag);
    println!("  digest:    sha256:{}", manifest.digest);
    println!("  entries:   {}", manifest.entries.len());

    let tar_bytes = tokio::fs::read(&out_tar)
        .await
        .map_err(|e| ArgoError::Provider(format!("read {}: {e}", out_tar.display())))?;
    let registry = OciDistributionRegistry::new(&host_base_url(&reference.host), oci_token());
    registry.push(&reference, &tar_bytes, &manifest).await?;
    println!("pushed {} ({} bytes) → {}", reference.to_canonical(), tar_bytes.len(), reference.host);
    Ok(())
}

async fn pull(cwd: &std::path::Path, oci_ref: &str) -> ArgoResult<()> {
    let reference = OciReference::parse(oci_ref)?;
    let registry = OciDistributionRegistry::new(&host_base_url(&reference.host), oci_token());
    let (tar_bytes, manifest) = registry.pull(&reference).await?;
    let out_tar = cwd.join(format!(
        "argo-pull-{}-{}.tar",
        reference.repo.replace('/', "-"),
        reference.tag
    ));
    tokio::fs::write(&out_tar, &tar_bytes)
        .await
        .map_err(|e| ArgoError::Provider(format!("write {}: {e}", out_tar.display())))?;
    println!(
        "pulled {} → {} ({} bytes)",
        reference.to_canonical(),
        out_tar.display(),
        tar_bytes.len()
    );
    println!("  workspace: {}", manifest.workspace_name);
    println!("  tag:       {}", manifest.tag);
    println!("  digest:    {}", manifest.digest);
    println!("  entries:   {}", manifest.entries.len());
    Ok(())
}

/// Determine the registry's HTTP base URL from the OCI host
/// (e.g. `ghcr.io` → `https://ghcr.io`; `localhost:5000` →
/// `http://localhost:5000` — useful for local dev registries
/// + the smoke-test mock).
fn host_base_url(host: &str) -> String {
    let scheme = if host.starts_with("localhost") || host.starts_with("127.0.0.1") {
        "http"
    } else {
        "https"
    };
    format!("{scheme}://{host}")
}

/// Read `ARGO_OCI_TOKEN` if set; sent as `Authorization: Bearer …`
/// on every registry request.
fn oci_token() -> Option<String> {
    std::env::var("ARGO_OCI_TOKEN").ok()
}
