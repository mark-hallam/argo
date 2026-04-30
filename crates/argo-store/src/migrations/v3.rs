//! Schema migration v3 — fix the FTS5 contentless-content bug.
//!
//! v1 declared `message_fts` with `content='message'` /
//! `content_rowid='rowid'`, which makes FTS5 try to read a column
//! named `content` from `message` on every search. The `message`
//! table has `content_json`, not `content`, so the JOIN fails
//! with "no such column: T.content" the first time anyone runs a
//! `MATCH` query.
//!
//! v3 drops and recreates `message_fts` as a **standalone** FTS5
//! table (no `content=` directive). The triggers we already had
//! already insert + delete the body into the FTS table on every
//! mutation, so the standalone form needs no other changes.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V3_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v3 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V3_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v3 down: {e}")))?;
    }
    Ok(())
}

const V3_UP: &[&str] = &[
    "DROP TRIGGER IF EXISTS message_fts_ad",
    "DROP TRIGGER IF EXISTS message_fts_ai",
    "DROP TABLE IF EXISTS message_fts",
    // Standalone FTS5 (no `content=` external-content link). The
    // body lives inside the FTS table; the existing triggers fan
    // it in on every INSERT.
    "CREATE VIRTUAL TABLE message_fts USING fts5(
        content,
        tokenize='unicode61 remove_diacritics 2'
    )",
    // Re-create the triggers to populate the standalone table.
    "CREATE TRIGGER message_fts_ai AFTER INSERT ON message BEGIN
        INSERT INTO message_fts(rowid, content) VALUES (new.rowid, new.content_json);
    END",
    "CREATE TRIGGER message_fts_ad AFTER DELETE ON message BEGIN
        DELETE FROM message_fts WHERE rowid = old.rowid;
    END",
    // Backfill any pre-existing message rows so search picks them
    // up. Empty for fresh DBs.
    "INSERT INTO message_fts (rowid, content) SELECT rowid, content_json FROM message",
];

const V3_DOWN: &[&str] = &[
    "DROP TRIGGER IF EXISTS message_fts_ad",
    "DROP TRIGGER IF EXISTS message_fts_ai",
    "DROP TABLE IF EXISTS message_fts",
    // Restore the (broken) v1 shape so `migrate_down_to(2)` is
    // truly the inverse of v3 up. Real users won't roll back
    // here; tests do.
    "CREATE VIRTUAL TABLE message_fts USING fts5(
        content,
        content='message',
        content_rowid='rowid',
        tokenize='unicode61 remove_diacritics 2'
    )",
    "CREATE TRIGGER message_fts_ai AFTER INSERT ON message BEGIN
        INSERT INTO message_fts(rowid, content) VALUES (new.rowid, new.content_json);
    END",
    "CREATE TRIGGER message_fts_ad AFTER DELETE ON message BEGIN
        INSERT INTO message_fts(message_fts, rowid, content) VALUES ('delete', old.rowid, old.content_json);
    END",
];
