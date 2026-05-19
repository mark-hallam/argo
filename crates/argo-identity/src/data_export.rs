//! Per-user data gathering for the customer-facing GDPR/CCPA Article
//! 15 export (Stream `mission-data-export-customer-flow`).
//!
//! Pure data layer — this module reads from the existing stores and
//! returns a structured [`ExportBundle`] of typed files. The gateway
//! handler is responsible for packing them into a `tar.gz` so this
//! crate stays tar-free.
//!
//! Scope is strict per-user (GDPR Article 15 boundary):
//! - `user_account.json` — the requesting user's own row.
//! - `tenants.json` — every `user_tenant` link row (id, role, joined_at)
//!   plus the resolved tenant `display_name`.
//! - `workers.jsonl` — workers in tenants where the user is `Owner`.
//!   Non-owner members get an empty list (the workers are tenant data,
//!   not user data; another tenant member would be the data subject).
//! - `audit/chain.jsonl` — `audit_record` rows whose `principal_id`
//!   matches `human:<email>` (lower-cased, exact match).
//! - `manifest.json` — version, scope, what's-included summary.
//!
//! Schema gaps documented in [`KNOWN_LIMITATIONS`]:
//! - memory + skills tables have no user-author column on the v1
//!   schema, so those sections ship an empty file plus an explanatory
//!   manifest entry. A v1.x follow-up (see brief 116) will widen the
//!   memory/skill schemas to attribute rows to the inserting user.
//! - outbound_message has no `sender` column — only `recipient` /
//!   `tenant_id` — so a true sender-scoped export is impossible until
//!   that schema lands.

use crate::user::{UserAccount, UserId, UserStatus};
use crate::user_tenant::UserTenantRole;
use crate::user_tenant_store::list_tenants_for_user;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::Serialize;
use serde_json::{json, Value};
use std::str::FromStr;
use uuid::Uuid;

/// Bumped whenever the on-disk layout of the bundle changes.
pub const EXPORT_FORMAT_VERSION: u32 = 1;

/// Files that are intentionally empty on the v1 schema, with the
/// reason surfaced in the bundle manifest.
pub const KNOWN_LIMITATIONS: &[(&str, &str)] = &[
    (
        "memory/rows.jsonl",
        "v1 schema: memory rows are tenant-scoped, not user-attributed; pending v1.x schema widen.",
    ),
    (
        "skills/",
        "v1 schema: skills are tenant-scoped, not user-attributed; pending v1.x schema widen.",
    ),
    (
        "messages/outbound.jsonl",
        "v1 schema: outbound_message has no sender column; only the user's owned-tenant rows ship.",
    ),
];

/// One file inside the tarball.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DataExportBundleFile {
    /// Path inside the tarball, relative to the bundle root.
    pub path: String,
    /// File contents — UTF-8 JSON / JSONL.
    pub contents: String,
}

/// Result of [`gather_for_user`]. Caller serialises [`Self::files`]
/// into the on-disk tarball (one tar entry per file).
#[derive(Debug, Clone)]
pub struct ExportBundle {
    /// All files for the bundle, including the manifest.
    pub files: Vec<DataExportBundleFile>,
    /// Total UTF-8 byte count of all files. Surfaced on the manifest
    /// and the polling-status response.
    pub total_bytes: usize,
}

/// Top-level façade: load the user's row, resolve their tenants, then
/// fan out into the per-section gatherers. The returned bundle is
/// suitable for the gateway handler to tar+gzip and persist.
pub async fn gather_for_user(
    store: &SessionStore,
    user_id: UserId,
    exported_at: DateTime<Utc>,
) -> ArgoResult<ExportBundle> {
    let user = load_user(store, user_id).await?;
    let tenants = list_tenants_for_user(store, user_id).await?;
    let owner_tenant_ids: Vec<TenantId> = tenants
        .iter()
        .filter(|t| matches!(t.role, UserTenantRole::Owner))
        .map(|t| t.tenant_id)
        .collect();

    let tenants_file = render_tenants_file(store, &tenants).await?;
    let workers_file = render_workers_file(store, &owner_tenant_ids).await?;
    let audit_file = render_audit_file(store, &user.email).await?;
    let outbound_file = render_outbound_file(store, &owner_tenant_ids).await?;
    let user_file = render_user_file(&user);

    let mut files = vec![
        user_file,
        tenants_file,
        workers_file,
        DataExportBundleFile {
            path: "audit/chain.jsonl".to_string(),
            contents: audit_file,
        },
        DataExportBundleFile {
            path: "memory/rows.jsonl".to_string(),
            contents: String::new(),
        },
        DataExportBundleFile {
            path: "skills/.empty".to_string(),
            contents: String::new(),
        },
        DataExportBundleFile {
            path: "messages/outbound.jsonl".to_string(),
            contents: outbound_file,
        },
    ];
    let total_bytes_before_manifest: usize = files.iter().map(|f| f.contents.len()).sum();
    let manifest = render_manifest(&user, &tenants, exported_at, total_bytes_before_manifest);
    files.insert(0, manifest);
    let total_bytes: usize = files.iter().map(|f| f.contents.len()).sum();
    Ok(ExportBundle { files, total_bytes })
}

async fn load_user(store: &SessionStore, user_id: UserId) -> ArgoResult<UserAccount> {
    crate::user_store::get_user(store, user_id)
        .await?
        .ok_or_else(|| ArgoError::Schema(format!("user {user_id} not found")))
}

fn render_user_file(user: &UserAccount) -> DataExportBundleFile {
    let payload = json!({
        "id": user.id.0.to_string(),
        "email": user.email,
        "display_name": user.display_name,
        "created_at": user.created_at,
        "last_login_at": user.last_login_at,
        "status": user.status.as_str(),
    });
    DataExportBundleFile {
        path: "user_account.json".to_string(),
        contents: format!("{payload:#}\n"),
    }
}

async fn render_tenants_file(
    store: &SessionStore,
    tenants: &[crate::user_tenant::UserTenant],
) -> ArgoResult<DataExportBundleFile> {
    let mut rows: Vec<Value> = Vec::with_capacity(tenants.len());
    for link in tenants {
        let name = lookup_tenant_display_name(store, link.tenant_id).await?;
        rows.push(json!({
            "tenant_id": link.tenant_id.0.to_string(),
            "display_name": name,
            "role": link.role.as_str(),
            "joined_at": link.created_at,
        }));
    }
    Ok(DataExportBundleFile {
        path: "tenants.json".to_string(),
        contents: format!(
            "{}\n",
            serde_json::to_string_pretty(&rows).unwrap_or_default()
        ),
    })
}

async fn lookup_tenant_display_name(
    store: &SessionStore,
    tenant_id: TenantId,
) -> ArgoResult<Option<String>> {
    let name: Option<String> = sqlx::query_scalar("SELECT name FROM tenant WHERE id = ?")
        .bind(tenant_id.0.as_bytes().as_slice())
        .fetch_optional(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("lookup tenant name: {e}")))?;
    Ok(name)
}

async fn render_workers_file(
    store: &SessionStore,
    owner_tenant_ids: &[TenantId],
) -> ArgoResult<DataExportBundleFile> {
    let mut lines = String::new();
    for tenant_id in owner_tenant_ids {
        let rows: Vec<(String, String, i64, Option<String>)> = sqlx::query_as(
            "SELECT id, vertical_id, created_at, archived_at \
             FROM worker WHERE tenant_id = ?",
        )
        .bind(tenant_id.to_string())
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("list workers: {e}")))?;
        for (id, vertical_id, created_at, archived_at) in rows {
            let line = json!({
                "tenant_id": tenant_id.0.to_string(),
                "worker_id": id,
                "vertical_id": vertical_id,
                "created_at": created_at,
                "archived_at": archived_at,
            });
            lines.push_str(&line.to_string());
            lines.push('\n');
        }
    }
    Ok(DataExportBundleFile {
        path: "workers.jsonl".to_string(),
        contents: lines,
    })
}

type AuditRow = (Vec<u8>, Option<Vec<u8>>, String, String, i64);

async fn render_audit_file(store: &SessionStore, email: &str) -> ArgoResult<String> {
    let principal = format!("human:{email}");
    let rows: Vec<AuditRow> = sqlx::query_as(
        "SELECT id, session_id, kind, payload_json, created_at \
         FROM audit_record WHERE principal_id = ? ORDER BY seq ASC",
    )
    .bind(&principal)
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("audit export: {e}")))?;
    let mut out = String::new();
    for (id_blob, session_blob, kind, payload, created_at) in rows {
        let line = json!({
            "id": Uuid::from_slice(&id_blob).map(|u| u.to_string()).unwrap_or_default(),
            "session_id": session_blob
                .and_then(|b| Uuid::from_slice(&b).ok())
                .map(|u| u.to_string()),
            "kind": kind,
            "payload": serde_json::from_str::<Value>(&payload).unwrap_or(Value::Null),
            "created_at": created_at,
        });
        out.push_str(&line.to_string());
        out.push('\n');
    }
    Ok(out)
}

type OutboundRow = (String, String, String, String, i64, String);

async fn render_outbound_file(
    store: &SessionStore,
    owner_tenant_ids: &[TenantId],
) -> ArgoResult<String> {
    if owner_tenant_ids.is_empty() {
        return Ok(String::new());
    }
    // The `outbound_message` table is bootstrapped lazily by
    // `argo_store::outbound_message_query::ensure_outbound_message_schema`;
    // workspaces that have never sent an outbound message yet won't
    // have the table on disk. Ensure it exists before we read so a
    // brand-new tenant's export reads as "no outbound rows" rather
    // than a hard error.
    argo_store::outbound_message_query::ensure_outbound_message_schema(store).await?;
    let mut out = String::new();
    for tenant_id in owner_tenant_ids {
        let rows: Vec<OutboundRow> = sqlx::query_as(
            "SELECT message_id, channel, recipient, body_hash, sent_at, outcome_json \
             FROM outbound_message WHERE tenant_id = ? ORDER BY sent_at ASC",
        )
        .bind(tenant_id.0.as_bytes().as_slice())
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("outbound export: {e}")))?;
        for (message_id, channel, recipient, body_hash, sent_at, outcome_json) in rows {
            let line = json!({
                "tenant_id": tenant_id.0.to_string(),
                "message_id": message_id,
                "channel": channel,
                "recipient": recipient,
                "body_hash": body_hash,
                "sent_at": sent_at,
                "outcome": serde_json::from_str::<Value>(&outcome_json).unwrap_or(Value::Null),
            });
            out.push_str(&line.to_string());
            out.push('\n');
        }
    }
    Ok(out)
}

fn render_manifest(
    user: &UserAccount,
    tenants: &[crate::user_tenant::UserTenant],
    exported_at: DateTime<Utc>,
    body_bytes: usize,
) -> DataExportBundleFile {
    let limitations: Vec<Value> = KNOWN_LIMITATIONS
        .iter()
        .map(|(path, reason)| json!({ "path": path, "reason": reason }))
        .collect();
    let payload = json!({
        "format_version": EXPORT_FORMAT_VERSION,
        "exported_at": exported_at.to_rfc3339_opts(SecondsFormat::Secs, true),
        "user_id": user.id.0.to_string(),
        "user_email": user.email,
        "tenant_count": tenants.len(),
        "body_bytes": body_bytes,
        "files_included": [
            "user_account.json",
            "tenants.json",
            "workers.jsonl",
            "audit/chain.jsonl",
            "memory/rows.jsonl",
            "skills/",
            "messages/outbound.jsonl",
        ],
        "scope": {
            "policy": "GDPR Article 15 / CCPA right-to-know",
            "boundary": "data where the requesting user is the subject (owner/author/principal)",
            "cross_user_excluded": "messages received from other users, audit rows attributed to other principals, tenant-level admin data unless the user is the tenant owner",
        },
        "known_limitations": limitations,
    });
    DataExportBundleFile {
        path: "manifest.json".to_string(),
        contents: format!(
            "{}\n",
            serde_json::to_string_pretty(&payload).unwrap_or_default()
        ),
    }
}

/// Test-helper: deserialise a `user_account.json` file emitted by
/// [`gather_for_user`] back into a [`UserAccount`]. The pretty-printed
/// `display_name` field can be `null`, so we tolerate that here.
#[doc(hidden)]
pub fn parse_user_file_for_test(json: &str) -> ArgoResult<UserAccount> {
    let v: Value = serde_json::from_str(json)
        .map_err(|e| ArgoError::Schema(format!("user_account parse: {e}")))?;
    let id_str = v
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| ArgoError::Schema("missing id".into()))?;
    let email = v
        .get("email")
        .and_then(Value::as_str)
        .ok_or_else(|| ArgoError::Schema("missing email".into()))?;
    let display_name = v
        .get("display_name")
        .and_then(Value::as_str)
        .map(str::to_string);
    let created_at = v
        .get("created_at")
        .and_then(Value::as_i64)
        .ok_or_else(|| ArgoError::Schema("missing created_at".into()))?;
    let last_login_at = v.get("last_login_at").and_then(Value::as_i64);
    let status_raw = v
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| ArgoError::Schema("missing status".into()))?;
    Ok(UserAccount {
        id: UserId(
            Uuid::parse_str(id_str).map_err(|e| ArgoError::Schema(format!("uuid parse: {e}")))?,
        ),
        email: email.to_string(),
        display_name,
        created_at,
        last_login_at,
        status: UserStatus::from_str(status_raw)
            .map_err(|e| ArgoError::Schema(format!("status parse: {e}")))?,
    })
}

#[cfg(test)]
#[path = "data_export_tests.rs"]
mod tests;
