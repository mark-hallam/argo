//! Workspace-scoped onboarding bookkeeping for the
//! `web-gateway-endpoints-onboarding-batch` brief.
//!
//! Two read/write surfaces — provider keys (encrypted at the handler
//! layer before reaching this module) and installed verticals. The
//! schema lives in `migrations/125__workspace_onboarding_keys.sql`;
//! [`ensure_schema`] applies the DDL idempotently on first use so
//! callers don't need to run the slot applier explicitly. Same
//! lazy-bootstrap pattern as `user_account::ensure_schema`.
//!
//! ```no_run
//! # async fn demo() -> argo_types::ArgoResult<()> {
//! use argo_store::workspace_keys::{ensure_schema, upsert_provider_key, ProviderKeyRecord};
//! use argo_types::{TenantId, WorkspaceId};
//! use sqlx::sqlite::SqlitePoolOptions;
//! let pool = SqlitePoolOptions::new().connect("sqlite::memory:").await.unwrap();
//! ensure_schema(&pool).await?;
//! upsert_provider_key(
//!     &pool,
//!     TenantId::new(),
//!     WorkspaceId::new(),
//!     ProviderKeyRecord {
//!         provider: "anthropic".into(),
//!         masked_key: "sk-ant-...abc1".into(),
//!         ciphertext_b64: "<age blob>".into(),
//!         test_cost_micros: 1_000,
//!         test_tokens: 4,
//!     },
//! )
//! .await?;
//! # Ok(()) }
//! ```

use argo_types::{ArgoError, ArgoResult, TenantId, WorkspaceId};
use sqlx::SqlitePool;

const ENSURE_SQL: &str = include_str!("../migrations/126__workspace_onboarding_keys.sql");

/// Apply the `workspace_provider_key` + `workspace_installed_vertical`
/// DDL idempotently. Safe to call on every gateway boot.
///
/// # Errors
/// Returns [`ArgoError::Provider`] if the underlying SQLite driver
/// rejects either `CREATE TABLE IF NOT EXISTS` statement.
pub async fn ensure_schema(pool: &SqlitePool) -> ArgoResult<()> {
    let stripped: String = ENSURE_SQL
        .lines()
        .filter(|line| !line.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n");
    for stmt in stripped.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        sqlx::query(stmt)
            .execute(pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("workspace_keys ensure: {e}")))?;
    }
    Ok(())
}

/// Insert-or-replace fields for `workspace_provider_key`. The
/// `ciphertext_b64` blob is opaque to this module — the gateway
/// handler wraps the raw API key with `argo-credentials::age_wrap`
/// before calling in.
#[derive(Debug, Clone)]
pub struct ProviderKeyRecord {
    /// Provider slug (`anthropic`, `openai`, `google`, `azure`, ...).
    pub provider: String,
    /// Display-safe masked form, e.g. `sk-ant-...abc1`.
    pub masked_key: String,
    /// Age-wrapped ciphertext (base64). Opaque to this module.
    pub ciphertext_b64: String,
    /// Reported cost of the validation call, in USD micros
    /// (1 USD = 1_000_000 micros).
    pub test_cost_micros: i64,
    /// Reported token usage of the validation call.
    pub test_tokens: i64,
}

/// One row returned by [`get_provider_key`] / [`list_provider_keys`].
#[derive(Debug, Clone)]
pub struct StoredProviderKey {
    /// Provider slug (`anthropic`, `openai`, `google`, `azure`, ...).
    pub provider: String,
    /// Display-safe masked form, e.g. `sk-ant-...abc1`.
    pub masked_key: String,
    /// Age-wrapped ciphertext (base64).
    pub ciphertext_b64: String,
    /// Reported cost of the validation call, in USD micros.
    pub test_cost_micros: i64,
    /// Reported token usage of the validation call.
    pub test_tokens: i64,
}

/// Insert or replace one provider key row for `(tenant, workspace, provider)`.
///
/// # Errors
/// Returns [`ArgoError::Provider`] on driver error.
pub async fn upsert_provider_key(
    pool: &SqlitePool,
    tenant: TenantId,
    workspace: WorkspaceId,
    rec: ProviderKeyRecord,
) -> ArgoResult<()> {
    let now = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO workspace_provider_key \
            (tenant_id, workspace_id, provider, masked_key, ciphertext_b64, \
             test_cost_micros, test_tokens, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) \
         ON CONFLICT(tenant_id, workspace_id, provider) DO UPDATE SET \
             masked_key = excluded.masked_key, \
             ciphertext_b64 = excluded.ciphertext_b64, \
             test_cost_micros = excluded.test_cost_micros, \
             test_tokens = excluded.test_tokens, \
             updated_at = excluded.updated_at",
    )
    .bind(tenant.0.as_bytes().as_slice())
    .bind(workspace.0.as_bytes().as_slice())
    .bind(&rec.provider)
    .bind(&rec.masked_key)
    .bind(&rec.ciphertext_b64)
    .bind(rec.test_cost_micros)
    .bind(rec.test_tokens)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("upsert_provider_key: {e}")))?;
    Ok(())
}

/// Fetch one provider key row scoped to `(tenant, workspace, provider)`.
///
/// # Errors
/// Returns [`ArgoError::Provider`] on driver error.
pub async fn get_provider_key(
    pool: &SqlitePool,
    tenant: TenantId,
    workspace: WorkspaceId,
    provider: &str,
) -> ArgoResult<Option<StoredProviderKey>> {
    let row: Option<(String, String, String, i64, i64)> = sqlx::query_as(
        "SELECT provider, masked_key, ciphertext_b64, test_cost_micros, test_tokens \
         FROM workspace_provider_key \
         WHERE tenant_id = ? AND workspace_id = ? AND provider = ?",
    )
    .bind(tenant.0.as_bytes().as_slice())
    .bind(workspace.0.as_bytes().as_slice())
    .bind(provider)
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("get_provider_key: {e}")))?;
    Ok(row.map(
        |(provider, masked_key, ciphertext_b64, test_cost_micros, test_tokens)| StoredProviderKey {
            provider,
            masked_key,
            ciphertext_b64,
            test_cost_micros,
            test_tokens,
        },
    ))
}

/// Record an installed vertical for `(tenant, workspace, vertical)`.
/// Idempotent — repeat calls refresh the workflow/skill counts and
/// the `installed_at` timestamp.
///
/// # Errors
/// Returns [`ArgoError::Provider`] on driver error.
pub async fn record_vertical_install(
    pool: &SqlitePool,
    tenant: TenantId,
    workspace: WorkspaceId,
    vertical_id: &str,
    workflows_added: i64,
    skills_added: i64,
) -> ArgoResult<()> {
    let now = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO workspace_installed_vertical \
            (tenant_id, workspace_id, vertical_id, workflows_added, skills_added, installed_at) \
         VALUES (?, ?, ?, ?, ?, ?) \
         ON CONFLICT(tenant_id, workspace_id, vertical_id) DO UPDATE SET \
             workflows_added = excluded.workflows_added, \
             skills_added = excluded.skills_added, \
             installed_at = excluded.installed_at",
    )
    .bind(tenant.0.as_bytes().as_slice())
    .bind(workspace.0.as_bytes().as_slice())
    .bind(vertical_id)
    .bind(workflows_added)
    .bind(skills_added)
    .bind(now)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("record_vertical_install: {e}")))?;
    Ok(())
}

/// List installed vertical ids for `(tenant, workspace)` in
/// install-order (most recent first).
///
/// # Errors
/// Returns [`ArgoError::Provider`] on driver error.
pub async fn list_installed_verticals(
    pool: &SqlitePool,
    tenant: TenantId,
    workspace: WorkspaceId,
) -> ArgoResult<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT vertical_id FROM workspace_installed_vertical \
         WHERE tenant_id = ? AND workspace_id = ? \
         ORDER BY installed_at DESC",
    )
    .bind(tenant.0.as_bytes().as_slice())
    .bind(workspace.0.as_bytes().as_slice())
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("list_installed_verticals: {e}")))?;
    Ok(rows.into_iter().map(|(v,)| v).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn pool() -> SqlitePool {
        let p = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        ensure_schema(&p).await.unwrap();
        p
    }

    #[tokio::test]
    async fn upsert_and_get_provider_key() {
        let p = pool().await;
        let tenant = TenantId::new();
        let workspace = WorkspaceId::new();
        upsert_provider_key(
            &p,
            tenant,
            workspace,
            ProviderKeyRecord {
                provider: "anthropic".into(),
                masked_key: "sk-ant-...abc1".into(),
                ciphertext_b64: "blob".into(),
                test_cost_micros: 1_000,
                test_tokens: 4,
            },
        )
        .await
        .unwrap();
        let got = get_provider_key(&p, tenant, workspace, "anthropic")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(got.masked_key, "sk-ant-...abc1");
        assert_eq!(got.ciphertext_b64, "blob");
        assert_eq!(got.test_cost_micros, 1_000);
        assert_eq!(got.test_tokens, 4);
    }

    #[tokio::test]
    async fn upsert_replaces_existing_row() {
        let p = pool().await;
        let tenant = TenantId::new();
        let workspace = WorkspaceId::new();
        for masked in ["sk-...one", "sk-...two"] {
            upsert_provider_key(
                &p,
                tenant,
                workspace,
                ProviderKeyRecord {
                    provider: "openai".into(),
                    masked_key: masked.into(),
                    ciphertext_b64: format!("blob-{masked}"),
                    test_cost_micros: 1,
                    test_tokens: 1,
                },
            )
            .await
            .unwrap();
        }
        let got = get_provider_key(&p, tenant, workspace, "openai")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(got.masked_key, "sk-...two");
        assert_eq!(got.ciphertext_b64, "blob-sk-...two");
    }

    #[tokio::test]
    async fn cross_tenant_isolation() {
        let p = pool().await;
        let alice = TenantId::new();
        let bob = TenantId::new();
        let ws = WorkspaceId::new();
        upsert_provider_key(
            &p,
            alice,
            ws,
            ProviderKeyRecord {
                provider: "anthropic".into(),
                masked_key: "sk-...alice".into(),
                ciphertext_b64: "blob".into(),
                test_cost_micros: 0,
                test_tokens: 0,
            },
        )
        .await
        .unwrap();
        let got = get_provider_key(&p, bob, ws, "anthropic").await.unwrap();
        assert!(got.is_none(), "bob must not see alice's key");
    }

    #[tokio::test]
    async fn install_and_list_verticals() {
        let p = pool().await;
        let tenant = TenantId::new();
        let workspace = WorkspaceId::new();
        record_vertical_install(&p, tenant, workspace, "gtm-engineer", 1, 5)
            .await
            .unwrap();
        record_vertical_install(&p, tenant, workspace, "sales-sdr", 0, 4)
            .await
            .unwrap();
        let installed = list_installed_verticals(&p, tenant, workspace).await.unwrap();
        assert_eq!(installed.len(), 2);
        assert!(installed.contains(&"gtm-engineer".to_string()));
        assert!(installed.contains(&"sales-sdr".to_string()));
    }

    #[tokio::test]
    async fn install_is_idempotent() {
        let p = pool().await;
        let tenant = TenantId::new();
        let workspace = WorkspaceId::new();
        for _ in 0..3 {
            record_vertical_install(&p, tenant, workspace, "gtm-engineer", 1, 5)
                .await
                .unwrap();
        }
        let installed = list_installed_verticals(&p, tenant, workspace).await.unwrap();
        assert_eq!(installed, vec!["gtm-engineer".to_string()]);
    }
}
