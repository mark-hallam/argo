//! `argo audit replay <session_id> [--from-seq N] [--until <iso>]
//! [--format pretty|raw]` (Stream `claude-audit-replay`).
//!
//! Walks one session's audit chain in seq order via
//! [`argo_audit::rows_for_session`], then renders through the
//! shared formatter ([`argo_audit::render_replay`]). The CLI
//! itself does no row-level work — the formatter + filter logic
//! lives in `argo-audit::replay` so the gateway can re-use it
//! later if a `/audit/replay` HTTP endpoint lands.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_audit::{filter_replay_rows, render_replay, rows_for_session, ReplayFormat};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use chrono::{DateTime, Utc};

/// Top-level entry. `format` is `pretty` (default) or `raw`.
pub async fn run(
    session_id: &str,
    from_seq: Option<i64>,
    until: Option<&str>,
    format: &str,
) -> ArgoResult<()> {
    let format = ReplayFormat::parse(format).ok_or_else(|| {
        ArgoError::Provider(format!(
            "unknown replay format '{format}': expected `pretty` or `raw`"
        ))
    })?;
    let until = match until {
        Some(s) => Some(parse_until(s)?),
        None => None,
    };
    let session_uuid = uuid::Uuid::parse_str(session_id)
        .map_err(|e| ArgoError::Schema(format!("session_id '{session_id}': {e}")))?;
    let session = SessionId(session_uuid);
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let rows = rows_for_session(&store, session).await?;
    let filtered = filter_replay_rows(rows, from_seq, until);
    let out = render_replay(&filtered, format);
    print!("{out}");
    Ok(())
}

fn parse_until(s: &str) -> ArgoResult<DateTime<Utc>> {
    s.parse::<DateTime<Utc>>()
        .map_err(|e| ArgoError::Schema(format!("--until '{s}': {e} (expected RFC-3339)")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_until_accepts_rfc3339() {
        let t = parse_until("2026-05-07T10:00:00Z").unwrap();
        assert_eq!(t.timestamp(), 1778148000);
    }

    #[test]
    fn parse_until_rejects_garbage() {
        assert!(parse_until("not-a-date").is_err());
        assert!(parse_until("").is_err());
    }
}
