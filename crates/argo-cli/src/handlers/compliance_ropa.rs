//! `argo compliance ropa` — GDPR Article 30 ROPA export.

use crate::cli::cli_compliance_cmds::CliComplianceRopaArgs;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_audit::{AuditKind, AuditSink};
use argo_capability::CapabilityEngine;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, Capability, Principal, TenantId};
use chrono::Utc;
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

const CLI_ROLE: &str = "owner";
const ROPA_CAPABILITY: &str = "compliance.ropa.export";

pub async fn run(args: CliComplianceRopaArgs) -> ArgoResult<()> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = Arc::new(SessionStore::open(db_path(&home)).await?);
    store.migrate().await?;

    let tenant = parse_tenant(&args.tenant)?;
    let principal = local_principal();
    require_ropa_export(&cwd, &principal)?;
    let exported_by = principal_label(&principal);
    let workspaces = load_workspaces(&store, tenant).await?;
    let report = RopaReport::new(tenant, exported_by.clone(), workspaces);
    write_report(&args.output, &report)?;
    emit_export_audit(&store, tenant, &exported_by, &args.output).await?;

    println!("wrote Article 30 ROPA export: {}", args.output);
    Ok(())
}

fn parse_tenant(value: &str) -> ArgoResult<TenantId> {
    Uuid::parse_str(value)
        .map(TenantId)
        .map_err(|e| ArgoError::Schema(format!("compliance ropa tenant '{value}': {e}")))
}

fn require_ropa_export(cwd: &Path, principal: &Principal) -> ArgoResult<()> {
    let engine = CapabilityEngine::with_workspace_defaults(cwd);
    engine.bind(principal.clone(), CLI_ROLE.to_string())?;
    engine.check(
        principal,
        &Capability::ToolDispatch {
            tool_name_glob: ROPA_CAPABILITY.to_string(),
        },
    )
}

fn local_principal() -> Principal {
    Principal::Local {
        os_user: exported_by_label(),
    }
}

fn exported_by_label() -> String {
    std::env::var("ARGO_PRINCIPAL")
        .or_else(|_| std::env::var("USER"))
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "operator".into())
}

fn principal_label(principal: &Principal) -> String {
    match principal {
        Principal::Local { os_user } => os_user.clone(),
        Principal::Gateway { adapter, identity } => format!("{adapter}:{identity}"),
        Principal::Acp { harness, identity } => format!("{harness}:{identity}"),
        Principal::Forge => "forge".into(),
    }
}

async fn load_workspaces(store: &SessionStore, tenant: TenantId) -> ArgoResult<Vec<RopaWorkspace>> {
    let rows: Vec<(Vec<u8>, String, String, i64)> = sqlx::query_as(
        "SELECT id, name, root_path, created_at FROM workspace \
         WHERE tenant_id = ? ORDER BY name ASC",
    )
    .bind(tenant.0.as_bytes().as_slice())
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("query ROPA workspace inventory: {e}")))?;

    rows.into_iter()
        .map(|(id, name, root_path, created_at)| {
            Ok(RopaWorkspace {
                id: uuid_from_blob(&id, "workspace.id")?,
                name,
                root_path,
                created_at,
            })
        })
        .collect()
}

fn uuid_from_blob(bytes: &[u8], column: &str) -> ArgoResult<String> {
    Uuid::from_slice(bytes)
        .map(|id| id.to_string())
        .map_err(|e| ArgoError::Invariant(format!("{column}: invalid uuid blob: {e}")))
}

fn write_report(output: &str, report: &RopaReport) -> ArgoResult<()> {
    let path = PathBuf::from(output);
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .map_err(|e| ArgoError::Io(format!("create {}: {e}", parent.display())))?;
    }
    let body = serde_json::to_string_pretty(report)
        .map_err(|e| ArgoError::Invariant(format!("serialize Article 30 ROPA: {e}")))?;
    std::fs::write(&path, body).map_err(|e| ArgoError::Io(format!("write {}: {e}", path.display())))
}

async fn emit_export_audit(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    exported_by: &str,
    output: &str,
) -> ArgoResult<()> {
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    sink.write(
        None,
        AuditKind::ComplianceRopaExported,
        json!({
            "tenant_id": tenant.to_string(),
            "exported_by": exported_by,
            "output_path_hash": hex::encode(Sha256::digest(output.as_bytes())),
        }),
        Some(exported_by.to_string()),
    )
    .await?;
    sink.flush_pending().await
}

#[derive(Serialize)]
struct RopaReport {
    schema: &'static str,
    article: &'static str,
    tenant_id: String,
    generated_at: String,
    exported_by: String,
    controller: RopaParty,
    processor: RopaParty,
    records: Vec<RopaRecord>,
    source_inventory: RopaInventory,
}

impl RopaReport {
    fn new(tenant: TenantId, exported_by: String, workspaces: Vec<RopaWorkspace>) -> Self {
        Self {
            schema: "argo.compliance.ropa.v1",
            article: "GDPR Article 30",
            tenant_id: tenant.to_string(),
            generated_at: Utc::now().to_rfc3339(),
            exported_by,
            controller: RopaParty {
                role: "controller",
                name: "Tenant controller",
                contact: "tenant-configured administrator",
            },
            processor: RopaParty {
                role: "processor",
                name: "Argo",
                contact: "operator security contact",
            },
            records: vec![RopaRecord::argo_operations()],
            source_inventory: RopaInventory { workspaces },
        }
    }
}

#[derive(Serialize)]
struct RopaParty {
    role: &'static str,
    name: &'static str,
    contact: &'static str,
}

#[derive(Serialize)]
struct RopaRecord {
    name: &'static str,
    purposes: Vec<&'static str>,
    data_subject_categories: Vec<&'static str>,
    personal_data_categories: Vec<&'static str>,
    recipient_categories: Vec<&'static str>,
    third_country_transfers: &'static str,
    retention_schedule: &'static str,
    security_measures: Vec<&'static str>,
}

impl RopaRecord {
    fn argo_operations() -> Self {
        Self {
            name: "Delegated AI operations",
            purposes: vec![
                "Operate tenant-authorized AI workers",
                "Maintain auditable task and approval history",
                "Support access and erasure workflows",
            ],
            data_subject_categories: vec![
                "workspace members",
                "operators",
                "contacts represented in tenant-provided task content",
            ],
            personal_data_categories: vec![
                "identity and account metadata",
                "workspace and task content",
                "audit metadata",
                "tenant-configured integration payloads",
            ],
            recipient_categories: vec![
                "tenant-authorized workspace members",
                "tenant-configured providers and connectors",
                "operator-approved support recipients",
            ],
            third_country_transfers: "Determined by tenant-selected providers and connectors.",
            retention_schedule:
                "Retained until tenant erasure, archive, or configured retention policy.",
            security_measures: vec![
                "tenant-scoped storage queries",
                "RBAC capability gate for export",
                "hash-chained audit row for each ROPA export",
                "audit payload omits raw output paths",
            ],
        }
    }
}

#[derive(Serialize)]
struct RopaInventory {
    workspaces: Vec<RopaWorkspace>,
}

#[derive(Serialize)]
struct RopaWorkspace {
    id: String,
    name: String,
    root_path: String,
    created_at: i64,
}
