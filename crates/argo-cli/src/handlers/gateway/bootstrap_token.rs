//! First-run setup-wizard bootstrap token.
//!
//! The token is stored under `~/.argo/.bootstrap-token` (or
//! `$ARGO_HOME/.bootstrap-token`) with Unix mode `0600`. Wizard
//! requests must carry `X-Argo-Bootstrap: <token>` until
//! `$ARGO_HOME/.wizard-done` exists, after which the legacy wizard is
//! permanently gone.

use crate::home::{ensure_dir, resolve_home};
use argo_audit::{AuditKind, AuditSink, SYSTEM_AUDIT_TENANT_ID};
use argo_types::{ArgoError, ArgoResult};
use rand_core::{OsRng, RngCore};
use std::path::PathBuf;
use std::sync::Arc;

const TOKEN_FILE: &str = ".bootstrap-token";
const WIZARD_DONE_FILE: &str = ".wizard-done";
const BOOTSTRAP_HEADER: &str = "X-Argo-Bootstrap";

pub(super) fn ensure_bootstrap_token() -> ArgoResult<String> {
    let path = token_path()?;
    if path.exists() {
        return read_token(&path);
    }
    write_new_token(&path)
}

pub(super) async fn rotate_command() -> ArgoResult<()> {
    let path = token_path()?;
    let token = write_new_token(&path)?;
    write_rotation_audit().await?;
    println!("gateway bootstrap token rotated: {}", path.display());
    println!("{token}");
    Ok(())
}

pub(super) fn is_wizard_request(req: &str) -> bool {
    request_path(req).is_some_and(|path| path.starts_with("/api/v1/wizard"))
}

pub(super) fn check_bootstrap_token(req: &str) -> Option<(&'static str, String)> {
    if !is_wizard_request(req) || req.starts_with("OPTIONS ") {
        return None;
    }
    match wizard_done_path() {
        Ok(path) if path.exists() => {
            return Some((
                "410 Gone",
                r#"{"error":"wizard_gone","reason":"first-run wizard already completed"}"#
                    .to_string(),
            ));
        }
        Ok(_) => {}
        Err(e) => return Some(server_error(&e)),
    }
    let expected = match ensure_bootstrap_token() {
        Ok(token) => token,
        Err(e) => return Some(server_error(&e)),
    };
    if presented_bootstrap(req).as_deref() == Some(expected.trim()) {
        None
    } else {
        Some((
            "401 Unauthorized",
            r#"{"error":"unauthorized","reason":"missing or invalid bootstrap token"}"#.to_string(),
        ))
    }
}

pub(super) fn wizard_options_response() -> (&'static str, String) {
    (
        "204 No Content\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: Content-Type, Authorization, X-Argo-Tenant-Id, X-Argo-Bootstrap\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS",
        String::new(),
    )
}

async fn write_rotation_audit() -> ArgoResult<()> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let store = super::boot::open_store(&cwd).await?;
    let sink = AuditSink::new_with_tenant(Arc::new(store), SYSTEM_AUDIT_TENANT_ID.to_string());
    sink.write(
        None,
        AuditKind::HttpRequest,
        serde_json::json!({
            "event_type": "gateway.bootstrap_token.rotate",
            "token_path": token_path()?.display().to_string(),
        }),
        Some("system:gateway-bootstrap".to_string()),
    )
    .await?;
    sink.flush_pending().await
}

fn token_path() -> ArgoResult<PathBuf> {
    Ok(home_dir()?.join(TOKEN_FILE))
}

fn wizard_done_path() -> ArgoResult<PathBuf> {
    Ok(home_dir()?.join(WIZARD_DONE_FILE))
}

fn home_dir() -> ArgoResult<PathBuf> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    Ok(home)
}

fn read_token(path: &PathBuf) -> ArgoResult<String> {
    std::fs::read_to_string(path)
        .map(|s| s.trim().to_string())
        .map_err(|e| ArgoError::Io(format!("read {}: {e}", path.display())))
}

fn write_new_token(path: &PathBuf) -> ArgoResult<String> {
    let token = generate_token();
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(path)
            .map_err(|e| ArgoError::Io(format!("open {}: {e}", path.display())))?;
        writeln!(file, "{token}")
            .map_err(|e| ArgoError::Io(format!("write {}: {e}", path.display())))?;
    }
    #[cfg(not(unix))]
    {
        std::fs::write(path, format!("{token}\n"))
            .map_err(|e| ArgoError::Io(format!("write {}: {e}", path.display())))?;
    }
    Ok(token)
}

fn generate_token() -> String {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

fn presented_bootstrap(req: &str) -> Option<String> {
    req.lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case(BOOTSTRAP_HEADER)
                .then(|| value.trim().to_string())
        })
        .filter(|value| !value.is_empty())
}

fn request_path(req: &str) -> Option<&str> {
    let (_method, rest) = req.split_once(' ')?;
    let path_end = rest.find([' ', '?']).unwrap_or(rest.len());
    Some(&rest[..path_end])
}

fn server_error(err: &ArgoError) -> (&'static str, String) {
    (
        "500 Internal Server Error",
        serde_json::json!({
            "error": "bootstrap_token_error",
            "reason": err.to_string(),
        })
        .to_string(),
    )
}
