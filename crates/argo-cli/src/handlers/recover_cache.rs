//! `argo recover-cache list/show/clear` — operator surface over the
//! `recovery_cache` table (#901) without invoking the LLM.

use crate::cli::cli_recover_cmds::{
    RecoverCacheClearCmd, RecoverCacheCmd, RecoverCacheListCmd, RecoverCacheShowCmd,
};
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::{recovery_cache, RecoveryCacheEntry, SessionStore};
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Utc};
use serde_json::json;

const SCHEMA_SQL: &str = include_str!("../../../argo-store/migrations/28__recovery_cache.sql");
const CONFIRM_TOKEN: &str = "yes-i-mean-it";

pub async fn run(cmd: RecoverCacheCmd) -> ArgoResult<()> {
    let store = open_store().await?;
    match cmd {
        RecoverCacheCmd::List(args) => list(&store, args).await,
        RecoverCacheCmd::Show(args) => show(&store, args).await,
        RecoverCacheCmd::Clear(args) => clear(&store, args).await,
    }
}

async fn open_store() -> ArgoResult<SessionStore> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    ensure_recovery_schema(&store).await?;
    Ok(store)
}

async fn ensure_recovery_schema(store: &SessionStore) -> ArgoResult<()> {
    for stmt in SCHEMA_SQL.split_terminator(';') {
        let sql = stmt.trim();
        if sql.is_empty() {
            continue;
        }
        sqlx::query(sql)
            .execute(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("recovery schema: {e}")))?;
    }
    Ok(())
}

async fn list(store: &SessionStore, args: RecoverCacheListCmd) -> ArgoResult<()> {
    let RecoverCacheListCmd { since, limit, json } = args;
    let since_micros = match since.as_deref() {
        Some(raw) => Some(parse_since_to_cutoff(raw)?),
        None => None,
    };
    let entries = recovery_cache::list(store.pool(), since_micros, limit).await?;
    if json {
        let envelope = json!({
            "ok": true,
            "kind": "recover_cache_list",
            "count": entries.len(),
            "since_micros": since_micros,
            "limit": limit,
            "entries": entries.iter().map(entry_to_json).collect::<Vec<_>>(),
        });
        let pretty = serde_json::to_string_pretty(&envelope)
            .map_err(|e| ArgoError::Provider(format!("serialize list envelope: {e}")))?;
        println!("{pretty}");
    } else if entries.is_empty() {
        println!("(no recovery cache entries)");
    } else {
        println!("session_id\t\t\t\t\tcached_at\t\tfailure_kind\tchain_hash");
        for e in &entries {
            println!(
                "{}\t{}\t{}\t{}",
                e.session_id,
                rfc3339(e.cached_at_micros),
                short(&e.failure_kind, 16),
                short(&e.chain_hash, 24)
            );
        }
    }
    Ok(())
}

async fn show(store: &SessionStore, args: RecoverCacheShowCmd) -> ArgoResult<()> {
    let RecoverCacheShowCmd { session_id, json } = args;
    match recovery_cache::show(store.pool(), &session_id).await? {
        Some((suggestions_json, chain_hash)) => {
            let parsed: serde_json::Value = serde_json::from_str(&suggestions_json)
                .map_err(|e| ArgoError::Provider(format!("decode cached suggestions: {e}")))?;
            if json {
                let envelope = json!({
                    "ok": true,
                    "kind": "recover_cache_show",
                    "session_id": session_id,
                    "chain_hash": chain_hash,
                    "from_cache": true,
                    "suggestions": parsed,
                });
                let pretty = serde_json::to_string_pretty(&envelope).map_err(|e| {
                    ArgoError::Provider(format!("serialize show envelope: {e}"))
                })?;
                println!("{pretty}");
            } else {
                println!("session_id: {session_id}");
                println!("chain_hash: {chain_hash}");
                println!("(cached — no LLM call)\n");
                println!(
                    "{}",
                    serde_json::to_string_pretty(&parsed).map_err(|e| ArgoError::Provider(
                        format!("re-serialize suggestions: {e}")
                    ))?
                );
            }
            Ok(())
        }
        None => Err(ArgoError::Provider(format!(
            "no cached suggestions for session '{session_id}'"
        ))),
    }
}

async fn clear(store: &SessionStore, args: RecoverCacheClearCmd) -> ArgoResult<()> {
    let RecoverCacheClearCmd {
        session_id,
        all,
        older_than,
        confirm,
        json,
    } = args;
    let removed: u64;
    let target: &str;
    if all {
        require_confirm(confirm.as_deref(), "wipe every recovery cache entry")?;
        removed = recovery_cache::clear_all(store.pool()).await?;
        target = "all";
    } else if let Some(duration) = older_than.as_deref() {
        let cutoff = parse_since_to_cutoff(duration)?;
        removed = recovery_cache::clear_older_than(store.pool(), cutoff).await?;
        target = duration;
    } else if let Some(sid) = session_id.as_deref() {
        removed = recovery_cache::clear_one(store.pool(), sid).await?;
        target = sid;
    } else {
        return Err(ArgoError::Provider(
            "argo recover-cache clear: pass <session-id>, --all, or --older-than <duration>".into(),
        ));
    }
    if json {
        let envelope = json!({
            "ok": true,
            "kind": "recover_cache_clear",
            "target": target,
            "removed": removed,
        });
        let pretty = serde_json::to_string_pretty(&envelope)
            .map_err(|e| ArgoError::Provider(format!("serialize clear envelope: {e}")))?;
        println!("{pretty}");
    } else if removed == 0 {
        println!("(no entries matched '{target}')");
    } else {
        println!("removed {removed} entries (target: {target})");
    }
    Ok(())
}

fn entry_to_json(e: &RecoveryCacheEntry) -> serde_json::Value {
    json!({
        "session_id": e.session_id,
        "chain_hash": e.chain_hash,
        "failure_kind": e.failure_kind,
        "cached_at_micros": e.cached_at_micros,
        "cached_at_rfc3339": rfc3339(e.cached_at_micros),
    })
}

fn rfc3339(micros: i64) -> String {
    DateTime::<Utc>::from_timestamp_micros(micros)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_default()
}

fn short(raw: &str, max: usize) -> String {
    if raw.is_empty() {
        return "-".into();
    }
    if raw.chars().count() <= max {
        return raw.to_string();
    }
    let head: String = raw.chars().take(max.saturating_sub(1)).collect();
    format!("{head}…")
}

fn require_confirm(provided: Option<&str>, action: &str) -> ArgoResult<()> {
    match provided {
        Some(token) if token == CONFIRM_TOKEN => Ok(()),
        _ => Err(ArgoError::Provider(format!(
            "{action} requires --confirm '{CONFIRM_TOKEN}' to proceed"
        ))),
    }
}

/// Parse a duration suffix (`7d`, `12h`, `30m`, `45s`) into the
/// absolute timestamp cutoff (`now - duration`). Used for both
/// `--since` (list filter) and `--older-than` (clear filter).
fn parse_since_to_cutoff(raw: &str) -> ArgoResult<i64> {
    let secs = parse_duration_secs(raw)?;
    let now = Utc::now().timestamp_micros();
    Ok(now - (secs as i64) * 1_000_000)
}

fn parse_duration_secs(raw: &str) -> ArgoResult<u64> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ArgoError::Provider("empty duration".into()));
    }
    let (num_part, unit) = trimmed.split_at(
        trimmed
            .find(|c: char| c.is_ascii_alphabetic())
            .unwrap_or(trimmed.len()),
    );
    let n: u64 = num_part
        .parse()
        .map_err(|e| ArgoError::Provider(format!("bad duration '{raw}': {e}")))?;
    let multiplier = match unit {
        "s" | "" => 1,
        "m" => 60,
        "h" => 3_600,
        "d" => 86_400,
        other => {
            return Err(ArgoError::Provider(format!(
                "unknown duration unit '{other}' — use s|m|h|d"
            )));
        }
    };
    Ok(n.saturating_mul(multiplier))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_duration_secs_supports_each_unit() {
        assert_eq!(parse_duration_secs("30s").unwrap(), 30);
        assert_eq!(parse_duration_secs("5m").unwrap(), 300);
        assert_eq!(parse_duration_secs("2h").unwrap(), 7200);
        assert_eq!(parse_duration_secs("7d").unwrap(), 7 * 86_400);
    }

    #[test]
    fn parse_duration_secs_rejects_unknown_unit() {
        assert!(parse_duration_secs("3y").is_err());
        assert!(parse_duration_secs("").is_err());
        assert!(parse_duration_secs("abc").is_err());
    }

    #[test]
    fn require_confirm_accepts_only_token() {
        assert!(require_confirm(Some("yes-i-mean-it"), "wipe").is_ok());
        assert!(require_confirm(Some("yes"), "wipe").is_err());
        assert!(require_confirm(None, "wipe").is_err());
    }

    #[test]
    fn short_truncates_with_ellipsis() {
        assert_eq!(short("", 8), "-");
        assert_eq!(short("abc", 8), "abc");
        assert_eq!(short("abcdefghijkl", 8), "abcdefg…");
    }

    #[test]
    fn rfc3339_round_trips_recent_timestamp() {
        let now = Utc::now().timestamp_micros();
        let formatted = rfc3339(now);
        assert!(formatted.contains('T'));
        assert!(formatted.ends_with('Z') || formatted.contains('+'));
    }
}
