//! Resolve a session-id argument that may be a full UUID or a
//! short prefix. CLI commands that take a session id (`show`,
//! `resume`, `chat`, `replay`, `steer`, …) call into this helper
//! before doing anything else, so the user can paste the first 8
//! chars of a UUID instead of the whole thing.
//!
//! Resolution rules:
//! 1. If `raw` parses as a full UUID → return it.
//! 2. Else query `session` for id-blobs whose lowercase-hex
//!    representation starts with `raw`. If exactly one matches,
//!    return its id. If zero or more than one, error with a
//!    helpful message.
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};

/// Try-parse `raw` as a full UUID; on failure, treat it as a
/// hex prefix and look it up in `session`. Returns the resolved
/// `SessionId` or a `Provider`-flavoured error explaining why
/// the prefix didn't resolve.
pub(crate) async fn resolve(store: &SessionStore, raw: &str) -> ArgoResult<SessionId> {
    if let Ok(u) = uuid::Uuid::parse_str(raw) {
        return Ok(SessionId(u));
    }
    let trimmed = raw.trim().to_lowercase();
    if trimmed.len() < 4 || !trimmed.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
        return Err(ArgoError::Provider(format!(
            "invalid session id '{raw}' — not a UUID and not a hex prefix"
        )));
    }
    // The id column is a 16-byte blob. SQLite's hex() emits upper-
    // case hex; we lower it here so the comparison is consistent
    // with our trimmed input.
    let pattern = format!("{}%", trimmed.replace('-', ""));
    let rows: Vec<Vec<u8>> = sqlx::query_scalar(
        "SELECT id FROM session WHERE lower(hex(id)) LIKE ? LIMIT 8",
    )
    .bind(&pattern)
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("session prefix lookup: {e}")))?;
    match rows.len() {
        0 => Err(ArgoError::Provider(format!(
            "no session matches prefix '{raw}'"
        ))),
        1 => {
            let bytes: [u8; 16] = rows[0].as_slice().try_into().map_err(|_| {
                ArgoError::Provider("session id row is not 16 bytes".into())
            })?;
            Ok(SessionId(uuid::Uuid::from_bytes(bytes)))
        }
        n => Err(ArgoError::Provider(format!(
            "ambiguous prefix '{raw}' — {n} sessions match (paste more chars)"
        ))),
    }
}
