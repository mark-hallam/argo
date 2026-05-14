//! `argo worker okr set/show/progress` (Stream
//! `claude-worker-okr-cli`, mission claim 2 worker-goals layer).
//!
//! KPI storage owns *observed values*; this handler owns *intent* —
//! quarterly OKRs declared per-worker. Progress joins each KR against
//! the latest `worker_kpi_value` row and emits a weighted score per
//! the brief formula.

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::worker_okr::{
    self, KeyResultProgress, KeyResultRow, KeyResultSpec, ObjectiveRow, ProgressReport,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use serde_json::json;

/// `argo worker okr set <worker_id> --period --title --kr ...`.
pub async fn set(
    worker_id: &str,
    tenant: &str,
    period: &str,
    title: &str,
    description: Option<&str>,
    kr_tokens: &[String],
    json_out: bool,
) -> ArgoResult<()> {
    let krs = kr_tokens
        .iter()
        .map(|t| KeyResultSpec::parse(t))
        .collect::<ArgoResult<Vec<_>>>()?;
    let pool = open_pool().await?;
    let id =
        worker_okr::set_objective(&pool, worker_id, tenant, period, title, description, &krs)
            .await?;
    if json_out {
        let payload = json!({
            "ok": true,
            "objective_id": id,
            "worker_id": worker_id,
            "tenant_id": tenant,
            "period": period,
            "title": title,
            "key_results": krs.iter().map(kr_spec_json).collect::<Vec<_>>(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
    } else {
        println!(
            "objective #{id}: {title} [{period}] — {n} key result(s)",
            n = krs.len()
        );
    }
    Ok(())
}

/// `argo worker okr show <worker_id> [--period] [--format markdown|json]`.
pub async fn show(
    worker_id: &str,
    tenant: &str,
    period: Option<&str>,
    format: &str,
) -> ArgoResult<()> {
    let pool = open_pool().await?;
    let objectives = worker_okr::list_objectives(&pool, worker_id, tenant, period).await?;
    let mut bundles: Vec<(ObjectiveRow, Vec<KeyResultRow>)> = Vec::with_capacity(objectives.len());
    for objective in objectives {
        let krs = worker_okr::key_results(&pool, objective.id).await?;
        bundles.push((objective, krs));
    }
    match format {
        "json" => {
            let payload = json!({
                "ok": true,
                "worker_id": worker_id,
                "tenant_id": tenant,
                "period": period,
                "count": bundles.len(),
                "objectives": bundles.iter().map(|(o, krs)| {
                    json!({
                        "id": o.id,
                        "period": o.period,
                        "title": o.title,
                        "description": o.description,
                        "status": o.status,
                        "created_at": o.created_at,
                        "key_results": krs.iter().map(kr_row_json).collect::<Vec<_>>(),
                    })
                }).collect::<Vec<_>>(),
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
            );
        }
        "markdown" => {
            print!("{}", render_markdown(worker_id, period, &bundles));
        }
        other => {
            return Err(ArgoError::Provider(format!(
                "invalid --format '{other}': expected markdown|json"
            )));
        }
    }
    Ok(())
}

/// `argo worker okr progress <worker_id> [--period] [--json]`.
pub async fn progress(
    worker_id: &str,
    tenant: &str,
    period: Option<&str>,
    json_out: bool,
) -> ArgoResult<()> {
    let pool = open_pool().await?;
    let reports = worker_okr::progress(&pool, worker_id, tenant, period).await?;
    if json_out {
        let payload = json!({
            "ok": true,
            "worker_id": worker_id,
            "tenant_id": tenant,
            "period": period,
            "count": reports.len(),
            "reports": reports.iter().map(report_json).collect::<Vec<_>>(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
    } else if reports.is_empty() {
        println!("(no objectives)");
    } else {
        for report in &reports {
            print_progress_block(report);
        }
    }
    Ok(())
}

fn render_markdown(
    worker_id: &str,
    period: Option<&str>,
    bundles: &[(ObjectiveRow, Vec<KeyResultRow>)],
) -> String {
    let mut out = String::new();
    let scope = period.map_or(String::from("all periods"), |p| format!("period {p}"));
    out.push_str(&format!("# Worker {worker_id} OKRs ({scope})\n\n"));
    if bundles.is_empty() {
        out.push_str("_No objectives recorded._\n");
        return out;
    }
    for (objective, krs) in bundles {
        out.push_str(&format!(
            "## {} — {} ({})\n",
            objective.period,
            objective.title,
            objective.status.as_str()
        ));
        if let Some(desc) = &objective.description {
            out.push_str(&format!("\n{desc}\n"));
        }
        out.push_str("\n| KPI | op | target | weight |\n");
        out.push_str("| --- | -- | -----: | -----: |\n");
        for kr in krs {
            out.push_str(&format!(
                "| {} | {} | {:.4} | {:.4} |\n",
                kr.kpi_name,
                kr.comparison.as_str(),
                kr.target_value,
                kr.weight,
            ));
        }
        out.push('\n');
    }
    out
}

fn print_progress_block(report: &ProgressReport) {
    let pct = report.weighted_score * 100.0;
    println!(
        "objective #{} [{}] {} — {:.1}% (status: {})",
        report.objective.id,
        report.objective.period,
        report.objective.title,
        pct,
        report.objective.status.as_str(),
    );
    println!("  {:<24}  {:<3}  {:>10}  {:>10}  {:>8}  status",
        "kpi",
        "op",
        "target",
        "actual",
        "score",
    );
    for kr in &report.key_results {
        let actual = kr
            .actual_value
            .map(|v| format!("{v:.4}"))
            .unwrap_or_else(|| "-".into());
        let status = if kr.achieved { "achieved" } else { "missed" };
        println!(
            "  {:<24}  {:<3}  {:>10.4}  {:>10}  {:>8.3}  {}",
            kr.key_result.kpi_name,
            kr.key_result.comparison.as_str(),
            kr.key_result.target_value,
            actual,
            kr.achievement,
            status,
        );
    }
}

fn kr_spec_json(spec: &KeyResultSpec) -> serde_json::Value {
    json!({
        "kpi": spec.kpi_name,
        "op": spec.comparison.as_str(),
        "target": spec.target_value,
        "weight": spec.weight,
    })
}

fn kr_row_json(row: &KeyResultRow) -> serde_json::Value {
    json!({
        "id": row.id,
        "kpi": row.kpi_name,
        "op": row.comparison.as_str(),
        "target": row.target_value,
        "weight": row.weight,
    })
}

fn report_json(report: &ProgressReport) -> serde_json::Value {
    json!({
        "objective_id": report.objective.id,
        "period": report.objective.period,
        "title": report.objective.title,
        "status": report.objective.status,
        "weighted_score": report.weighted_score,
        "key_results": report.key_results.iter().map(progress_kr_json).collect::<Vec<_>>(),
    })
}

fn progress_kr_json(kr: &KeyResultProgress) -> serde_json::Value {
    json!({
        "kpi": kr.key_result.kpi_name,
        "op": kr.key_result.comparison.as_str(),
        "target": kr.key_result.target_value,
        "weight": kr.key_result.weight,
        "actual": kr.actual_value,
        "achievement": kr.achievement,
        "achieved": kr.achieved,
    })
}

async fn open_pool() -> ArgoResult<sqlx::SqlitePool> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    Ok(store.pool().clone())
}

#[cfg(test)]
#[path = "worker_okr_tests.rs"]
mod tests;
