//! `argo demo seed` — invoke [`argo_demo_seed::run_canonical_v1_seed`].
//!
//! Reads the local workspace home, opens the SQLite session store,
//! runs migrations, and dispatches the canonical v1 demo seed body.
//! Prints a Pass A / Pass B / approval / cross-tenant transcript at
//! the end so the operator can paste the output into the demo
//! recording PR body.

use std::sync::Arc;
use std::time::Instant;

use argo_demo_seed::{run_canonical_v1_seed, SeedError, SeedOptions, SeedReport, DEFAULT_PROFILE};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use uuid::Uuid;

use crate::home::{db_path, ensure_dir, resolve_home};

/// Inputs accepted by `argo demo seed`. Mirrors the CLI flag set.
#[derive(Debug, Clone)]
pub struct DemoSeedArgs {
    /// Tenant id the seed runs under. Must already be provisioned.
    pub tenant_id: String,
    /// Profile id. Defaults to the canonical v1 profile.
    pub profile: String,
    /// Number of Pass A turn-shaped audit rows to insert.
    pub pass_a_turns: usize,
    /// Number of Pass B dressing rows to insert.
    pub dressing_rows: usize,
    /// Skip Pass A entirely (no provider key required).
    pub skip_pass_a: bool,
    /// Output the seed report as JSON instead of human-readable text.
    pub json: bool,
}

/// Entry point invoked by `main_dispatch_*::demo`.
pub async fn run(args: DemoSeedArgs) -> ArgoResult<()> {
    let tenant_uuid = Uuid::parse_str(&args.tenant_id)
        .map_err(|e| ArgoError::Schema(format!("--tenant: not a UUID: {e}")))?;
    let tenant_id = TenantId(tenant_uuid);
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let store = Arc::new(store);

    let opts = SeedOptions {
        tenant_id,
        profile: args.profile.clone(),
        pass_a_turns: args.pass_a_turns,
        dressing_audit_rows: args.dressing_rows,
        skip_pass_a_turns: args.skip_pass_a,
        actor: "argo-demo-seed".to_string(),
    };

    let started = Instant::now();
    let report = match run_canonical_v1_seed(&store, &opts).await {
        Ok(r) => r,
        Err(SeedError::MissingProviderKey) => {
            return Err(ArgoError::Provider(format!(
                "demo seed Pass A requires the `{}` env var; configure a sandbox \
                 provider key or re-run with `--skip-pass-a-turns` to seed Pass B \
                 dressing rows only",
                argo_demo_seed::PROVIDER_KEY_ENV
            )));
        }
        Err(SeedError::UnknownProfile(p)) => {
            return Err(ArgoError::Schema(format!(
                "demo seed: unknown profile `{p}` (known: `{DEFAULT_PROFILE}`)"
            )));
        }
        Err(SeedError::Argo(e)) => return Err(e),
    };
    let elapsed = started.elapsed();

    if args.json {
        let body = serde_json::json!({
            "tenant_id": args.tenant_id,
            "profile": args.profile,
            "elapsed_ms": elapsed.as_millis(),
            "report": report,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&body)
                .map_err(|e| ArgoError::Schema(format!("seed report serialize: {e}")))?
        );
    } else {
        print_text_report(&args, &report, elapsed);
    }
    Ok(())
}

fn print_text_report(args: &DemoSeedArgs, report: &SeedReport, elapsed: std::time::Duration) {
    println!("argo demo seed — profile `{}`", args.profile);
    println!("  tenant_id:                  {}", args.tenant_id);
    println!("  elapsed:                    {:.2}s", elapsed.as_secs_f64());
    println!();
    println!("Pass A — function-side (real backend rows):");
    println!("  workers_created:            {}", report.workers_created);
    println!("  workflows_created:          {}", report.workflows_created);
    println!("  skills_created:             {}", report.skills_created);
    println!("  pass_a_real_llm:            {}", report.pass_a_real_llm);
    println!("  pass_a_audit_rows:          {}", report.pass_a_audit_rows);
    println!();
    println!("Pass B — dressing (fixture-tier rows the camera does not drill into):");
    println!("  pass_b_audit_rows:          {}", report.pass_b_audit_rows);
    println!("  approval_requests:          {}", report.approval_requests);
    println!();
    println!("Cross-tenant invisibility (Scene 6 negative check):");
    println!("  foreign_workers_created:    {}", report.foreign_workers_created);
    println!("  foreign_skills_created:     {}", report.foreign_skills_created);
    println!("  foreign_audit_rows:         {}", report.foreign_audit_rows);
    println!();
    if !report.pass_a_real_llm {
        println!(
            "note: Pass A real-LLM turns were skipped (--skip-pass-a-turns); \
             scaffolding rows were inserted but the run_turn body is deferred to \
             `mission-demo-seed-real-llm-driver`."
        );
    }
}
