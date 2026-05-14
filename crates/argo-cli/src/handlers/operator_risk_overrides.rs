//! `argo operator risk-overrides set/get/list/delete` (Stream
//! `mission-operator-specific-risk-overrides`, MISSION_BACKLOG
//! Section 3 Claim 6 #122). Per-operator overrides of the
//! manifest-driven default risk classification live in the workspace
//! `operator_risk_overrides` table; the resolver in
//! `argo_channels::operator_risk_overrides::RiskResolver` consults
//! the row before falling through to the default policy.

use crate::cli::cli_operator_cmds::{OperatorCmd, OperatorRiskOverridesCmd, RiskLevelArg};
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::operator_risk_overrides::{
    OperatorRiskOverrideRow, OperatorRiskOverridesStore,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::Utc;
use serde::Serialize;
use serde_json::json;

/// Single-entry dispatch from `main.rs` over the whole
/// `argo operator …` subcommand tree. Keeps the top-level dispatch
/// table to one line per command, leaving the 500-LOC ceiling on
/// `main.rs` un-pressured as the tree grows.
pub async fn dispatch(cmd: OperatorCmd) -> ArgoResult<()> {
    match cmd {
        OperatorCmd::RiskOverrides { cmd } => run(cmd).await,
    }
}

pub async fn run(cmd: OperatorRiskOverridesCmd) -> ArgoResult<()> {
    match cmd {
        OperatorRiskOverridesCmd::Set {
            operator,
            action_kind,
            risk_level,
            vertical,
            allow_downgrade,
            json,
        } => set(&operator, &action_kind, risk_level, vertical.as_deref(), allow_downgrade, json).await,
        OperatorRiskOverridesCmd::Get { operator, json } => get(&operator, json).await,
        OperatorRiskOverridesCmd::List { json } => list(json).await,
        OperatorRiskOverridesCmd::Delete {
            operator,
            action_kind,
            vertical,
            json,
        } => delete(&operator, &action_kind, vertical.as_deref(), json).await,
    }
}

async fn set(
    operator: &str,
    action_kind: &str,
    risk_level: RiskLevelArg,
    vertical: Option<&str>,
    allow_downgrade: bool,
    json_out: bool,
) -> ArgoResult<()> {
    let store = open_store().await?;
    let now = Utc::now().timestamp();
    OperatorRiskOverridesStore::new(store.pool())
        .set(
            operator,
            action_kind,
            vertical,
            risk_level.as_str(),
            allow_downgrade,
            now,
        )
        .await?;
    let row = OperatorRiskOverrideRow {
        operator_id: operator.to_string(),
        action_kind: action_kind.to_string(),
        vertical_id: vertical.map(String::from),
        risk_level: risk_level.as_str().to_string(),
        allow_downgrade,
        created_at: now,
    };
    if json_out {
        print_json(&json!({"ok": true, "override": to_dto(&row)}))
    } else {
        println!(
            "set operator_risk_overrides operator={} action_kind={} vertical={} \
             risk_level={} allow_downgrade={} created_at={}",
            row.operator_id,
            row.action_kind,
            row.vertical_id.as_deref().unwrap_or("(all)"),
            row.risk_level,
            row.allow_downgrade,
            row.created_at,
        );
        Ok(())
    }
}

async fn get(operator: &str, json_out: bool) -> ArgoResult<()> {
    let store = open_store().await?;
    let rows = OperatorRiskOverridesStore::new(store.pool())
        .get_for_operator(operator)
        .await?;
    if json_out {
        let body: Vec<_> = rows.iter().map(to_dto).collect();
        print_json(&json!({"operator": operator, "overrides": body}))
    } else if rows.is_empty() {
        println!("(no risk overrides for operator {operator})");
        Ok(())
    } else {
        print_table(&rows);
        Ok(())
    }
}

async fn list(json_out: bool) -> ArgoResult<()> {
    let store = open_store().await?;
    let rows = OperatorRiskOverridesStore::new(store.pool()).list().await?;
    if json_out {
        let body: Vec<_> = rows.iter().map(to_dto).collect();
        print_json(&json!({"overrides": body}))
    } else if rows.is_empty() {
        println!("(no operator risk overrides)");
        Ok(())
    } else {
        print_table(&rows);
        Ok(())
    }
}

async fn delete(
    operator: &str,
    action_kind: &str,
    vertical: Option<&str>,
    json_out: bool,
) -> ArgoResult<()> {
    let store = open_store().await?;
    let removed = OperatorRiskOverridesStore::new(store.pool())
        .delete(operator, action_kind, vertical)
        .await?;
    if json_out {
        print_json(&json!({
            "ok": true,
            "removed": removed,
            "operator": operator,
            "action_kind": action_kind,
            "vertical": vertical,
        }))
    } else if removed {
        println!(
            "deleted operator_risk_overrides operator={operator} action_kind={action_kind} \
             vertical={}",
            vertical.unwrap_or("(all)")
        );
        Ok(())
    } else {
        println!(
            "(no override for operator {operator} action_kind {action_kind} vertical {})",
            vertical.unwrap_or("(all)")
        );
        Ok(())
    }
}

fn print_table(rows: &[OperatorRiskOverrideRow]) {
    println!(
        "{:<24}  {:<28}  {:<16}  {:<8}  {:<6}  created_at",
        "operator_id", "action_kind", "vertical_id", "risk", "downgr"
    );
    for r in rows {
        println!(
            "{:<24}  {:<28}  {:<16}  {:<8}  {:<6}  {}",
            r.operator_id,
            r.action_kind,
            r.vertical_id.as_deref().unwrap_or("(all)"),
            r.risk_level,
            r.allow_downgrade,
            r.created_at,
        );
    }
}

async fn open_store() -> ArgoResult<SessionStore> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    Ok(store)
}

#[derive(Serialize)]
struct OverrideDto {
    operator_id: String,
    action_kind: String,
    vertical_id: Option<String>,
    risk_level: String,
    allow_downgrade: bool,
    created_at: i64,
}

fn to_dto(r: &OperatorRiskOverrideRow) -> OverrideDto {
    OverrideDto {
        operator_id: r.operator_id.clone(),
        action_kind: r.action_kind.clone(),
        vertical_id: r.vertical_id.clone(),
        risk_level: r.risk_level.clone(),
        allow_downgrade: r.allow_downgrade,
        created_at: r.created_at,
    }
}

fn print_json<T: Serialize>(value: &T) -> ArgoResult<()> {
    let body = serde_json::to_string_pretty(value)
        .map_err(|e| ArgoError::Provider(format!("serialize JSON: {e}")))?;
    println!("{body}");
    Ok(())
}
