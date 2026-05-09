//! Shared helpers for `migration_reconciliation_tests.rs`. Split
//! out of the test file to keep it under the 500-LOC ceiling once
//! the v34..v40 sub-brief expanded the assertion lists.
use crate::SessionStore;
use sqlx::Row;

pub(super) async fn baseline_v11() -> SessionStore {
    let store = SessionStore::open(":memory:").await.unwrap();
    crate::migrations::run_up_to(store.pool(), 11)
        .await
        .unwrap();
    store
}

pub(super) async fn apply_raw(store: &SessionStore, body: &str) {
    // Strip `--` line comments before splitting on `;` so semicolons
    // inside SQL prose comments (e.g. `19__worker_fs_root.sql`) do
    // not split mid-statement.
    let stripped: String = body
        .lines()
        .map(|line| match line.find("--") {
            Some(idx) => &line[..idx],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n");
    for stmt in split_with_trigger_awareness(&stripped) {
        match sqlx::query(stmt).execute(store.pool()).await {
            Ok(_) => {}
            Err(e)
                if stmt.contains("memory_vec_index")
                    && e.to_string().contains("no such module: vec0") => {}
            Err(e) => panic!("raw migration failed: {e}; stmt={stmt}"),
        }
    }
}

// Mirrors `tests/migration_chain.rs::split_sql_statements`: splits on
// `;` but treats a statement starting with `CREATE TRIGGER` as
// continuing until its `END;` so the inner `;` inside the
// `BEGIN ... END` body do not slice the trigger in half. Required
// for `22__tenant_archived.sql` once it is part of the v22..v40
// reconciliation walk.
fn split_with_trigger_awareness(sql: &str) -> Vec<&str> {
    let mut statements = Vec::new();
    let mut start = 0;
    let bytes = sql.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b';' {
            let stmt = sql[start..=index].trim();
            if !is_incomplete_trigger(stmt) {
                if !stmt.is_empty() {
                    statements.push(stmt);
                }
                start = index + 1;
            }
        }
        index += 1;
    }
    let tail = sql[start..].trim();
    if !tail.is_empty() {
        statements.push(tail);
    }
    statements
        .into_iter()
        .map(|s| s.trim_end_matches(';').trim())
        .filter(|s| !s.is_empty())
        .collect()
}

fn is_incomplete_trigger(stmt: &str) -> bool {
    stmt.trim_start()
        .get(..14)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("CREATE TRIGGER"))
        && !ends_with_end(stmt)
}

fn ends_with_end(stmt: &str) -> bool {
    let stmt = stmt.trim_end_matches(';').trim_end();
    stmt.get(stmt.len().saturating_sub(3)..)
        .is_some_and(|suffix| suffix.eq_ignore_ascii_case("END"))
}

pub(super) async fn table_exists(store: &SessionStore, table: &str) -> bool {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type IN ('table','view') AND name = ?",
    )
    .bind(table)
    .fetch_optional(store.pool())
    .await
    .unwrap();
    row.is_some()
}

pub(super) async fn assert_columns(store: &SessionStore, table: &str, columns: &[&str]) {
    let rows = sqlx::query(&format!("PRAGMA table_info({table})"))
        .fetch_all(store.pool())
        .await
        .unwrap();
    for column in columns {
        assert!(
            rows.iter()
                .any(|row| row.get::<String, _>("name") == *column),
            "missing {table}.{column}"
        );
    }
}

pub(super) async fn assert_schema_version_singleton(store: &SessionStore) {
    let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM schema_version")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(rows, 1);
}
