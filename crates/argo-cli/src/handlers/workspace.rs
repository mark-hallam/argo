//! `argo workspace` — bundle init / push / pull / diff.
use crate::cli::WorkspaceCmd;
use argo_marketplace::{build_bundle, OciReference};
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
        WorkspaceCmd::Init => init(&cwd),
        WorkspaceCmd::Push { oci_ref } => push(&cwd, &oci_ref).await,
        WorkspaceCmd::Pull { oci_ref } => pull(&oci_ref),
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

fn init(cwd: &std::path::Path) -> ArgoResult<()> {
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
    println!("workspace initialized at {}", cwd.display());
    println!("  + argo.toml");
    println!("  + skills/, personas/, cron/, rubrics/");
    println!("  + MEMORY.md, USER.md, SOUL.md, AGENTS.md");
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
    println!("note: actual OCI registry push lands with the §23.3 follow-up.");
    Ok(())
}

fn pull(oci_ref: &str) -> ArgoResult<()> {
    // Parse-only path until §23.3 wires the real client. This is
    // still useful: rejects malformed refs with a clean error.
    let _ = OciReference::parse(oci_ref)?;
    println!("workspace pull from {oci_ref}: registry client lands with §23.3.");
    Ok(())
}
