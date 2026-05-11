//! `argo genesis from-vertical` handler — Mission
//! `mission-genesis-transfer-from-tenant`.
//!
//! Wraps `argo_genesis::plan_from_seed_vertical_in_default_root`
//! with cli-specific concerns: reads the charter from disk (with a
//! safety cap), maps `GenesisError` → `ArgoError`, and prints the
//! resulting seeded plan as either text or JSON.
//!
//! Lives in its own file so `handlers/genesis.rs` stays under the
//! 500-LOC per-file ceiling.

use argo_genesis::{
    plan_from_seed_vertical_in_default_root, seeded_from, write_vertical, GenesisPlan,
};
use argo_types::{ArgoError, ArgoResult};
use std::path::Path;

/// Charter files larger than this byte cap are tail-truncated.
/// Mirrors the implicit 5 MiB cap on the URL-fetch path in
/// `from-job-description`; the much smaller value here is fine
/// because a charter is operator-authored prose, not a scraped page.
const CHARTER_FILE_CAP_BYTES: usize = 4 * 1024;

pub(crate) fn handle_from_vertical(
    charter_file: &Path,
    seed: &str,
    persona: &str,
    output: &Path,
    json: bool,
) -> ArgoResult<()> {
    let charter = read_charter(charter_file)?;
    let plan = plan_from_seed_vertical_in_default_root(&charter, persona, seed)
        .map_err(ArgoError::from)?;
    let abs_output = output.to_path_buf();
    write_vertical(&plan, &abs_output).map_err(ArgoError::from)?;
    print_seeded_plan(&plan, &abs_output, seed, json);
    Ok(())
}

fn read_charter(charter_file: &Path) -> ArgoResult<String> {
    let body = std::fs::read_to_string(charter_file).map_err(|e| {
        ArgoError::Provider(format!(
            "could not read charter file '{}': {e}",
            charter_file.display()
        ))
    })?;
    if body.len() <= CHARTER_FILE_CAP_BYTES {
        return Ok(body);
    }
    // UTF-8-safe truncation: walk back to a char boundary.
    let mut end = CHARTER_FILE_CAP_BYTES;
    while end > 0 && !body.is_char_boundary(end) {
        end -= 1;
    }
    Ok(body[..end].to_string())
}

fn print_seeded_plan(plan: &GenesisPlan, output: &Path, seed: &str, json: bool) {
    if json {
        let payload = serde_json::json!({
            "vertical_id": plan.vertical_id,
            "persona": plan.persona,
            "archetype": plan.archetype,
            "charter": plan.charter,
            "output_dir": output.display().to_string(),
            "dry_run_hours": argo_genesis::DRY_RUN_HOURS,
            "action_count": plan
                .manifest
                .actions
                .as_ref()
                .map(|a| a.actions.len())
                .unwrap_or(0),
            "seeded_from": seeded_from(&plan.manifest).unwrap_or(seed),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
    } else {
        println!(
            "Generated vertical (seeded from {seed}): {}",
            plan.vertical_id
        );
        println!("  archetype:   {:?}", plan.archetype);
        println!("  persona:     {}", plan.persona);
        println!("  output:      {}", output.display());
        println!(
            "  actions:     {}",
            plan.manifest
                .actions
                .as_ref()
                .map(|a| a.actions.len())
                .unwrap_or(0)
        );
        println!(
            "  seeded_from: {}",
            seeded_from(&plan.manifest).unwrap_or(seed)
        );
        println!(
            "Next: review prompts/system.md (the seed contributed action vocab — \
             prune what doesn't fit), then `argo vertical lint`."
        );
    }
}
