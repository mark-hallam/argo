//! `argo contract apply/validate/show` (Stream
//! `claude-delegation-contract-template`, V1.1 mission claim 2).
//!
//! A `<role>.contract.toml` file is the operator-facing artefact that
//! captures the full delegation: charter, duties, KPI definitions,
//! quarterly objectives + key results, and per-period success criteria.
//! `apply` is a thin orchestrator that materialises the file into the
//! existing identity / OKR stores (no new schema). `validate` is a
//! schema-only check (no DB). `show` reads a worker's current state
//! and renders it back out as TOML.
//!
//! Persistence stays in the existing primitives:
//!   - charter → `IdentityStore::update_charter` (markdown body)
//!   - duties + KPI definitions + success criteria + role label →
//!     bundled into the worker row's `kpis` JSON blob (free-form)
//!   - objectives + key results → `argo_store::worker_okr::set_objective`
//!
//! KPI *values* (time-series observations) still flow through
//! `argo worker kpi record`. Apply never writes a value row.
//!
//! Schema types, parser, validator, and render helpers live in the
//! sibling [`super::contract_schema`] module.

use crate::handlers::contract_schema::{
    build_kpis_blob, kr_entry_to_spec, read_and_validate, render_contract, ApplyPlan,
};
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_identity::{IdentityStore, MAX_CHARTER_BYTES, MAX_KPIS_BYTES, WorkerId};
use argo_store::worker_okr::{self, KeyResultRow, ObjectiveRow};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde_json::{json, Value};
use std::path::Path;
use std::str::FromStr;
use uuid::Uuid;

/// `argo contract apply <file> [--tenant <t>] [--worker <id>] [--dry-run]`.
pub async fn apply(
    file: &Path,
    tenant: Option<&str>,
    worker_id: Option<&str>,
    dry_run: bool,
    json_out: bool,
) -> ArgoResult<()> {
    let contract = read_and_validate(file)?;
    let plan = ApplyPlan::from(&contract);
    if dry_run {
        emit_dry_run(file, worker_id, &plan, json_out);
        return Ok(());
    }
    let worker_raw = worker_id.ok_or_else(|| {
        ArgoError::Schema(
            "--worker <id> is required for `contract apply` \
             (creation stays via `argo worker create`)"
                .into(),
        )
    })?;
    let tenant_raw = tenant.ok_or_else(|| {
        ArgoError::Schema("--tenant <id> is required for `contract apply`".into())
    })?;
    let tenant_id = parse_tenant(tenant_raw)?;
    let worker = parse_worker(worker_raw)?;

    let charter_md = contract.contract.charter.clone();
    enforce_charter_size(&charter_md)?;
    let kpis_value = build_kpis_blob(&contract);
    enforce_kpis_size(&kpis_value)?;

    let store = open_store().await?;
    IdentityStore::new(&store)
        .update_charter(tenant_id, worker, Some(&charter_md), Some(&kpis_value))
        .await?;

    let pool = store.pool().clone();
    let mut objective_ids: Vec<i64> = Vec::with_capacity(contract.objectives.len());
    for obj in &contract.objectives {
        let krs = obj
            .key_results
            .iter()
            .map(kr_entry_to_spec)
            .collect::<ArgoResult<Vec<_>>>()?;
        let id = worker_okr::set_objective(
            &pool,
            worker_raw,
            tenant_raw,
            &obj.period,
            &obj.title,
            obj.description.as_deref(),
            &krs,
        )
        .await?;
        objective_ids.push(id);
    }
    print_apply_outcome(worker_raw, tenant_raw, &charter_md, &contract, &objective_ids, json_out);
    Ok(())
}

/// `argo contract validate <file>` — schema check only, no DB.
pub async fn validate(file: &Path, json_out: bool) -> ArgoResult<()> {
    let contract = read_and_validate(file)?;
    let plan = ApplyPlan::from(&contract);
    if json_out {
        let payload = json!({
            "ok": true,
            "file": file.display().to_string(),
            "role": contract.contract.role,
            "charter_bytes": contract.contract.charter.len(),
            "duties": plan.duty_count,
            "kpis": plan.kpi_count,
            "objectives": plan.objective_count,
            "key_results": plan.kr_count,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
    } else {
        println!("ok: {} parses cleanly", file.display());
        println!("  role: {}", contract.contract.role);
        println!("  charter: {} bytes", contract.contract.charter.len());
        println!("  duties: {}", plan.duty_count);
        println!("  kpis: {}", plan.kpi_count);
        println!(
            "  objectives: {} ({} key-results total)",
            plan.objective_count, plan.kr_count
        );
    }
    Ok(())
}

/// `argo contract show <worker-id> --tenant <t>` — render the current
/// state back out as TOML.
pub async fn show(worker_id: &str, tenant: &str, json_out: bool) -> ArgoResult<()> {
    let tenant_id = parse_tenant(tenant)?;
    let worker = parse_worker(worker_id)?;
    let store = open_store().await?;
    let row = IdentityStore::new(&store)
        .get_worker(tenant_id, worker)
        .await?
        .ok_or_else(|| {
            ArgoError::Provider(format!(
                "worker {worker_id} not found in tenant {tenant}"
            ))
        })?;
    let pool = store.pool().clone();
    let objectives = worker_okr::list_objectives(&pool, worker_id, tenant, None).await?;
    let mut bundles: Vec<(ObjectiveRow, Vec<KeyResultRow>)> = Vec::with_capacity(objectives.len());
    for obj in objectives {
        let krs = worker_okr::key_results(&pool, obj.id).await?;
        bundles.push((obj, krs));
    }
    let contract = render_contract(&row.charter, row.kpis.as_ref(), &bundles, worker_id);
    if json_out {
        println!(
            "{}",
            serde_json::to_string_pretty(&contract).unwrap_or_else(|_| "{}".into())
        );
    } else {
        let toml = toml::to_string_pretty(&contract)
            .map_err(|e| ArgoError::Provider(format!("contract serialise: {e}")))?;
        print!("{toml}");
    }
    Ok(())
}

fn emit_dry_run(file: &Path, worker_id: Option<&str>, plan: &ApplyPlan, json_out: bool) {
    if json_out {
        let payload = json!({
            "ok": true,
            "dry_run": true,
            "file": file.display().to_string(),
            "worker_id": worker_id,
            "would": {
                "update_charter": true,
                "upsert_kpis_blob": true,
                "duties": plan.duty_count,
                "kpis": plan.kpi_count,
                "set_objectives": plan.objective_count,
                "set_key_results": plan.kr_count,
            },
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
    } else {
        let target = worker_id.unwrap_or("(no --worker; would refuse without --dry-run)");
        println!("dry-run: {} → worker {}", file.display(), target);
        println!("  would update charter (markdown)");
        println!(
            "  would upsert kpis blob (duties={}, kpis={}, success_criteria included)",
            plan.duty_count, plan.kpi_count,
        );
        println!(
            "  would set {} objective(s) and {} key-result(s)",
            plan.objective_count, plan.kr_count
        );
    }
}

fn print_apply_outcome(
    worker_raw: &str,
    tenant_raw: &str,
    charter_md: &str,
    contract: &super::contract_schema::ContractFile,
    objective_ids: &[i64],
    json_out: bool,
) {
    if json_out {
        let payload = json!({
            "ok": true,
            "worker_id": worker_raw,
            "tenant_id": tenant_raw,
            "applied": {
                "charter_bytes": charter_md.len(),
                "duties": contract.duties.len(),
                "kpis": contract.kpis.len(),
                "objectives": objective_ids.len(),
                "objective_ids": objective_ids,
            },
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
    } else {
        println!("contract applied to worker {worker_raw}:");
        println!("  charter: {} bytes", charter_md.len());
        println!("  duties: {}", contract.duties.len());
        println!("  kpis (definitions): {}", contract.kpis.len());
        println!(
            "  objectives: {} (#{})",
            objective_ids.len(),
            objective_ids
                .iter()
                .map(i64::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
}

fn parse_tenant(tenant: &str) -> ArgoResult<TenantId> {
    Uuid::parse_str(tenant)
        .map(TenantId)
        .map_err(|e| ArgoError::Schema(format!("invalid tenant id '{tenant}': {e}")))
}

fn parse_worker(worker_id: &str) -> ArgoResult<WorkerId> {
    WorkerId::from_str(worker_id)
        .map_err(|e| ArgoError::Schema(format!("invalid worker id '{worker_id}': {e}")))
}

async fn open_store() -> ArgoResult<SessionStore> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    Ok(store)
}

fn enforce_charter_size(body: &str) -> ArgoResult<()> {
    if body.len() > MAX_CHARTER_BYTES {
        return Err(ArgoError::Schema(format!(
            "charter exceeds {MAX_CHARTER_BYTES} bytes (got {})",
            body.len()
        )));
    }
    Ok(())
}

fn enforce_kpis_size(value: &Value) -> ArgoResult<()> {
    let serialised = serde_json::to_string(value)
        .map_err(|e| ArgoError::Provider(format!("serialise kpis: {e}")))?;
    if serialised.len() > MAX_KPIS_BYTES {
        return Err(ArgoError::Schema(format!(
            "kpis exceed {MAX_KPIS_BYTES} bytes (got {})",
            serialised.len()
        )));
    }
    Ok(())
}

#[cfg(test)]
#[path = "contract_tests.rs"]
mod tests;
