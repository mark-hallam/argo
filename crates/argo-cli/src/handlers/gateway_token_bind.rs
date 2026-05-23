//! `argo gateway token bind|unbind|list` — operator-facing
//! management of the `service_token_binding` table that anchors
//! gateway bearer tokens to a `(principal_id, tenant_id)` pair
//! (Mission `mission-audit-principal-tenant-from-token`, P0).
//!
//! The raw bearer token is never accepted on the command line and is
//! never persisted: the CLI reads the token from `--token-file
//! <path>` (or `ARGO_GATEWAY_BIND_TOKEN` env var) and only the
//! SHA-256 hex digest is written to the store. `list` projects the
//! first eight hex characters as a fingerprint so operators can
//! correlate bindings to the matching token without exposing the
//! hash itself.
use crate::cli::cli_gateway_cmds::GatewayTokenCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::service_token_binding::{
    delete_binding, insert_binding, list_bindings, TokenBinding,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::path::Path;

const ENV_TOKEN: &str = "ARGO_GATEWAY_BIND_TOKEN";

/// Entry point dispatched from `argo gateway token <…>`.
pub async fn run(cmd: GatewayTokenCmd) -> ArgoResult<()> {
    let store = open_store().await?;
    match cmd {
        GatewayTokenCmd::Bind {
            token_file,
            principal,
            tenant,
            label,
        } => bind(&store, token_file.as_deref(), &principal, &tenant, label.as_deref()).await,
        GatewayTokenCmd::Unbind { token_file } => unbind(&store, token_file.as_deref()).await,
        GatewayTokenCmd::List { json } => list(&store, json).await,
    }
}

async fn bind(
    store: &SessionStore,
    token_file: Option<&Path>,
    principal: &str,
    tenant: &str,
    label: Option<&str>,
) -> ArgoResult<()> {
    let token = read_token(token_file)?;
    let hash = sha256_hex(&token);
    let created_at = Utc::now().to_rfc3339();
    insert_binding(store.pool(), &hash, principal, tenant, label, &created_at).await?;
    println!(
        "bound token (fingerprint {}) → principal={principal} tenant={tenant}",
        fingerprint(&hash)
    );
    Ok(())
}

async fn unbind(store: &SessionStore, token_file: Option<&Path>) -> ArgoResult<()> {
    let token = read_token(token_file)?;
    let hash = sha256_hex(&token);
    let removed = delete_binding(store.pool(), &hash).await?;
    if removed {
        println!("unbound token (fingerprint {})", fingerprint(&hash));
    } else {
        println!("no binding found (fingerprint {})", fingerprint(&hash));
    }
    Ok(())
}

async fn list(store: &SessionStore, json: bool) -> ArgoResult<()> {
    let rows = list_bindings(store.pool()).await?;
    if json {
        print_json(&rows)
    } else {
        print_table(&rows);
        Ok(())
    }
}

fn read_token(token_file: Option<&Path>) -> ArgoResult<String> {
    if let Some(path) = token_file {
        let raw = std::fs::read_to_string(path)
            .map_err(|e| ArgoError::Io(format!("read token file {}: {e}", path.display())))?;
        return Ok(raw.trim().to_string());
    }
    // tenant-unscoped: single-tenant CLI invocation
    match argo_config::cred::get_unscoped(ENV_TOKEN) {
        Ok(t) if !t.is_empty() => Ok(t),
        _ => Err(ArgoError::Schema(format!(
            "provide --token-file <path> or set {ENV_TOKEN}; tokens are never accepted on argv"
        ))),
    }
}

fn sha256_hex(s: &str) -> String {
    let digest = Sha256::digest(s.as_bytes());
    let mut out = String::with_capacity(digest.len() * 2);
    for b in digest {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

fn fingerprint(hash: &str) -> String {
    hash.chars().take(8).collect()
}

async fn open_store() -> ArgoResult<SessionStore> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    Ok(store)
}

fn print_table(rows: &[TokenBinding]) {
    if rows.is_empty() {
        println!("(no bindings)");
        return;
    }
    println!("{:<10} {:<32} {:<38} {:<16} CREATED_AT", "FP", "PRINCIPAL", "TENANT", "LABEL");
    for r in rows {
        println!(
            "{:<10} {:<32} {:<38} {:<16} {}",
            fingerprint(&r.token_hash),
            truncate(&r.principal_id, 32),
            r.tenant_id,
            r.label.as_deref().unwrap_or("-"),
            r.created_at,
        );
    }
}

fn print_json(rows: &[TokenBinding]) -> ArgoResult<()> {
    let payload: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "fingerprint": fingerprint(&r.token_hash),
                "principal": r.principal_id,
                "tenant": r.tenant_id,
                "label": r.label,
                "created_at": r.created_at,
            })
        })
        .collect();
    let out = serde_json::to_string_pretty(&serde_json::json!({ "bindings": payload }))
        .map_err(|e| ArgoError::Provider(format!("token-list json: {e}")))?;
    println!("{out}");
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}
