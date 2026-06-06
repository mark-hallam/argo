//! Schema migration v91 — the `executor_promotion` table backing the
//! per-action execution-graduation path (UF Phase-B · B3, DEC-037).
//!
//! A hired teammate is **draft-only at birth** ([[DEC-025]] #6): every
//! generated action materialises as `executor = "human"`, so a human
//! carries it out offline. DEC-037 greenlit the deliberate post-hire
//! grant — a **proven** teammate may graduate one action from
//! draft-only to actually executing it via a real executor (MCP today).
//! Promotion is **per-action, opt-in, RBAC-gated, after probation**;
//! every promotion is an audit row, and approval STAYS on sensitive
//! verbs (promotion changes *who* executes after approval, never that
//! approval is required).
//!
//! This table is the durable home for those grants. A row exists only
//! when an operator has explicitly promoted `(tenant, worker, action)`
//! off the human default; its absence means "still draft-only", which
//! is the v0 behaviour every workspace ships with.
//!
//! ```text
//! executor_promotion(
//!   tenant_id     TEXT NOT NULL,   -- tenant boundary (string TenantId)
//!   worker_id     TEXT NOT NULL,   -- WorkerId string form (matches worker_freeze)
//!   action_type   TEXT NOT NULL,   -- the manifest action_type being graduated
//!   executor_kind TEXT NOT NULL,   -- 'mcp' (the only promotable kind in v1)
//!   mcp_server    TEXT,            -- MCP server name (required when kind='mcp')
//!   mcp_tool      TEXT,            -- MCP tool name within the server
//!   promoted_by   TEXT NOT NULL,   -- principal id that granted the promotion
//!   promoted_at   TEXT NOT NULL,   -- RFC3339 grant timestamp
//!   PRIMARY KEY (tenant_id, worker_id, action_type)
//! )
//! ```
//!
//! The composite primary key `(tenant_id, worker_id, action_type)` is
//! also the read path: dispatch looks up exactly one row per action and
//! the tenant column is always bound, so a promotion can never bleed
//! across tenants.
//!
//! Additive + idempotent: `CREATE TABLE IF NOT EXISTS` only, no existing
//! column touched, so the migration is safe to re-run and safe to roll
//! forward over any prior schema version. Inline SQL mirrors the
//! v87/v89/v90 pattern (no `migrations/NNN__*.sql` slot file, so no
//! filesystem slot to collide with — the `duplicate_slot_validator`
//! that runs at boot stays happy).

use argo_types::ArgoError;
use sqlx::Sqlite;

const V91_UP_TABLE_SQL: &str = "CREATE TABLE IF NOT EXISTS executor_promotion (\
    tenant_id     TEXT NOT NULL,\
    worker_id     TEXT NOT NULL,\
    action_type   TEXT NOT NULL,\
    executor_kind TEXT NOT NULL,\
    mcp_server    TEXT,\
    mcp_tool      TEXT,\
    promoted_by   TEXT NOT NULL,\
    promoted_at   TEXT NOT NULL,\
    PRIMARY KEY (tenant_id, worker_id, action_type)\
)";

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    sqlx::query(V91_UP_TABLE_SQL)
        .execute(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("v91 up table: {e}")))?;
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    sqlx::query("DROP TABLE IF EXISTS executor_promotion")
        .execute(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("v91 down: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn up_creates_executor_promotion_table() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("pool");

        // Run the full versioned runner up to and including v91 so the
        // migration is exercised exactly as the runtime applies it.
        super::super::run_up_to(&pool, 91).await.expect("migrations");

        sqlx::query(
            "INSERT INTO executor_promotion \
             (tenant_id, worker_id, action_type, executor_kind, mcp_server, mcp_tool, \
              promoted_by, promoted_at) \
             VALUES ('tenant-a', 'worker-1', 'send_email', 'mcp', 'gmail', 'drafts.send', \
                     'principal:owner', '2026-06-06T00:00:00+00:00')",
        )
        .execute(&pool)
        .await
        .expect("insert into executor_promotion");

        let tool: String = sqlx::query_scalar(
            "SELECT mcp_tool FROM executor_promotion WHERE tenant_id = 'tenant-a'",
        )
        .fetch_one(&pool)
        .await
        .expect("read back row");
        assert_eq!(tool, "drafts.send");
    }

    #[tokio::test]
    async fn primary_key_is_tenant_worker_action() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("pool");
        super::super::run_up_to(&pool, 91).await.expect("migrations");

        // Same (tenant, worker, action) in two tenants is two distinct
        // rows — the tenant column is part of the key.
        for tenant in ["tenant-a", "tenant-b"] {
            sqlx::query(
                "INSERT INTO executor_promotion \
                 (tenant_id, worker_id, action_type, executor_kind, mcp_server, mcp_tool, \
                  promoted_by, promoted_at) \
                 VALUES (?, 'worker-1', 'send_email', 'mcp', 'gmail', 'drafts.send', \
                         'principal:owner', '2026-06-06T00:00:00+00:00')",
            )
            .bind(tenant)
            .execute(&pool)
            .await
            .expect("insert per tenant");
        }
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM executor_promotion")
            .fetch_one(&pool)
            .await
            .expect("count");
        assert_eq!(count, 2, "one promotion row per tenant");
    }

    #[tokio::test]
    async fn down_drops_the_table() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("pool");
        super::super::run_up_to(&pool, 91).await.expect("up to v91");

        super::super::roll_back_to(&pool, 90)
            .await
            .expect("roll back to v90");

        let exists: Option<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'table' \
             AND name = 'executor_promotion'",
        )
        .fetch_optional(&pool)
        .await
        .expect("probe sqlite_master");
        assert!(exists.is_none(), "table should be dropped by v91 down");
    }
}
