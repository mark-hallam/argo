//! Prometheus text-format helpers for `metrics.rs`. Sibling so
//! `metrics.rs` stays under the per-file 500-LOC budget.
//!
//! Each helper turns a single SQL aggregate into a `# HELP` /
//! `# TYPE` / `<name> <value>` triplet. SQL errors surface as
//! `# error: …` comment lines + a `0` fallback so a partial DB
//! never breaks the whole scrape — same contract as the
//! pre-split inline helpers.

use std::fmt::Write;

/// Run a single-row `COUNT(*)` (or similar scalar `i64` query)
/// and write a Prometheus counter triplet into `out`.
pub(super) async fn push_count(
    out: &mut String,
    pool: &sqlx::SqlitePool,
    name: &str,
    help: &str,
    sql: &str,
) {
    let _ = writeln!(out, "\n# HELP {name} {help}\n# TYPE {name} counter");
    match sqlx::query_scalar::<_, i64>(sql).fetch_one(pool).await {
        Ok(n) => {
            let _ = writeln!(out, "{name} {n}");
        }
        Err(e) => {
            let _ = writeln!(out, "# error: {e}\n{name} 0");
        }
    }
}

/// Sibling of `push_count` for `SUM(...)` aggregates. Same
/// triplet shape; separate fn so the call sites read clearly
/// against the SQL they wrap.
pub(super) async fn push_sum(
    out: &mut String,
    pool: &sqlx::SqlitePool,
    name: &str,
    help: &str,
    sql: &str,
) {
    let _ = writeln!(out, "\n# HELP {name} {help}\n# TYPE {name} counter");
    match sqlx::query_scalar::<_, i64>(sql).fetch_one(pool).await {
        Ok(n) => {
            let _ = writeln!(out, "{name} {n}");
        }
        Err(e) => {
            let _ = writeln!(out, "# error: {e}\n{name} 0");
        }
    }
}

/// `argo_forge_runs_total{status="..."}` — group-by counter
/// with one labelled line per terminal status. Empty result
/// emits a single `status="none"` line so the metric always
/// appears in the scrape.
pub(super) async fn push_forge_runs_by_status(out: &mut String, pool: &sqlx::SqlitePool) {
    let _ = writeln!(
        out,
        "\n# HELP argo_forge_runs_total Recorded forge runs grouped by terminal status\n\
         # TYPE argo_forge_runs_total counter"
    );
    let rows: Result<Vec<(String, i64)>, _> =
        sqlx::query_as("SELECT status, COUNT(*) FROM forge_run GROUP BY status")
            .fetch_all(pool)
            .await;
    match rows {
        Ok(rows) if rows.is_empty() => {
            let _ = writeln!(out, "argo_forge_runs_total{{status=\"none\"}} 0");
        }
        Ok(rows) => {
            for (status, n) in rows {
                let safe = status.replace('"', "");
                let _ = writeln!(out, "argo_forge_runs_total{{status=\"{safe}\"}} {n}");
            }
        }
        Err(e) => {
            let _ = writeln!(
                out,
                "# error: forge_run group-by status failed: {e}\n\
                 argo_forge_runs_total{{status=\"unknown\"}} 0"
            );
        }
    }
}
