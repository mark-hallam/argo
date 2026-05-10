//! Schema setup, FTS scoring, and embedding refresh for the
//! three-layer memory store. Sibling of [`crate::layers`]; split from
//! it under the 500-LOC ceiling.
use crate::embedding::{vector_to_json, EmbeddingProvider};
use crate::MemoryStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::Utc;
use sqlx::Row;
use std::collections::HashMap;

impl MemoryStore {
    /// Ensure the additive layer/namespace/index schema exists.
    pub(crate) async fn ensure_layer_schema(&self) -> ArgoResult<()> {
        let pool = self.pool();
        let _sqlite_vec_init: unsafe extern "C" fn() = sqlite_vec::sqlite3_vec_init;
        if !memory_column_exists(pool, "layer").await? {
            exec_schema(
                pool,
                "ALTER TABLE memory_entry ADD COLUMN layer TEXT NOT NULL DEFAULT 'recent'",
            )
            .await?;
        }
        if !memory_column_exists(pool, "tenant_id").await? {
            exec_schema(pool, "ALTER TABLE memory_entry ADD COLUMN tenant_id BLOB").await?;
        }
        if !memory_column_exists(pool, "worker_id").await? {
            exec_schema(pool, "ALTER TABLE memory_entry ADD COLUMN worker_id BLOB").await?;
        }
        if !memory_column_exists(pool, "memory_scope").await? {
            exec_schema(
                pool,
                "ALTER TABLE memory_entry ADD COLUMN memory_scope TEXT",
            )
            .await?;
        }
        exec_schema(
            pool,
            "CREATE INDEX IF NOT EXISTS idx_memory_entry_layer \
             ON memory_entry(workspace_id, layer, learned_at)",
        )
        .await?;
        exec_schema(
            pool,
            "CREATE INDEX IF NOT EXISTS idx_memory_entry_ns_layer \
             ON memory_entry(tenant_id, worker_id, memory_scope, layer, learned_at)",
        )
        .await?;
        exec_schema(
            pool,
            "CREATE VIRTUAL TABLE IF NOT EXISTS memory_entry_fts USING fts5(\
             entry_id UNINDEXED, body, tokenize='unicode61 remove_diacritics 2')",
        )
        .await?;
        exec_schema(
            pool,
            "CREATE TABLE IF NOT EXISTS memory_embedding (\
             entry_id BLOB PRIMARY KEY REFERENCES memory_entry(id) ON DELETE CASCADE, \
             model TEXT NOT NULL, dims INTEGER NOT NULL, vector_json TEXT NOT NULL, \
             created_at INTEGER NOT NULL)",
        )
        .await?;
        let _ = sqlx::query(
            "CREATE VIRTUAL TABLE IF NOT EXISTS memory_vec_index USING vec0(\
             entry_id TEXT PRIMARY KEY, embedding FLOAT[64])",
        )
        .execute(pool)
        .await;
        Ok(())
    }

    pub(crate) async fn refresh_indexed_embeddings(
        &self,
        provider: &dyn EmbeddingProvider,
    ) -> ArgoResult<()> {
        self.ensure_layer_schema().await?;
        let rows = sqlx::query("SELECT id, body FROM memory_entry WHERE layer = 'indexed'")
            .fetch_all(self.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("memory indexed scan: {e}")))?;
        for row in rows {
            let id: Vec<u8> = row
                .try_get("id")
                .map_err(|e| ArgoError::Provider(format!("memory indexed id: {e}")))?;
            let body: String = row
                .try_get("body")
                .map_err(|e| ArgoError::Provider(format!("memory indexed body: {e}")))?;
            let vector = provider.embed(&body)?;
            let vector_json = vector_to_json(&vector)?;
            let entry_hex = hex::encode_upper(&id);
            sqlx::query("DELETE FROM memory_entry_fts WHERE entry_id = ?")
                .bind(&entry_hex)
                .execute(self.pool())
                .await
                .map_err(|e| ArgoError::Provider(format!("memory fts delete: {e}")))?;
            sqlx::query("INSERT INTO memory_entry_fts(entry_id, body) VALUES (?, ?)")
                .bind(&entry_hex)
                .bind(&body)
                .execute(self.pool())
                .await
                .map_err(|e| ArgoError::Provider(format!("memory fts insert: {e}")))?;
            sqlx::query(
                "INSERT INTO memory_embedding (entry_id, model, dims, vector_json, created_at) \
                 VALUES (?, ?, ?, ?, ?) \
                 ON CONFLICT(entry_id) DO UPDATE SET model = excluded.model, \
                 dims = excluded.dims, vector_json = excluded.vector_json, \
                 created_at = excluded.created_at",
            )
            .bind(id.as_slice())
            .bind(provider.model())
            .bind(vector.len() as i64)
            .bind(vector_json)
            .bind(Utc::now().timestamp_micros())
            .execute(self.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("memory embedding upsert: {e}")))?;
        }
        Ok(())
    }

    pub(crate) async fn fts_scores(&self, query: &str) -> ArgoResult<HashMap<String, f64>> {
        let Some(fts) = fts_query(query) else {
            return Ok(HashMap::new());
        };
        let rows = sqlx::query(
            "SELECT entry_id, bm25(memory_entry_fts) AS rank \
             FROM memory_entry_fts WHERE memory_entry_fts MATCH ?",
        )
        .bind(fts)
        .fetch_all(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("memory fts query: {e}")))?;
        let mut out = HashMap::new();
        for row in rows {
            let entry_id: String = row
                .try_get("entry_id")
                .map_err(|e| ArgoError::Provider(format!("memory fts entry_id: {e}")))?;
            let rank: f64 = row
                .try_get("rank")
                .map_err(|e| ArgoError::Provider(format!("memory fts rank: {e}")))?;
            out.insert(entry_id, rank);
        }
        Ok(out)
    }
}

async fn memory_column_exists(pool: &sqlx::Pool<sqlx::Sqlite>, column: &str) -> ArgoResult<bool> {
    let rows = sqlx::query("PRAGMA table_info(memory_entry)")
        .fetch_all(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("memory table_info: {e}")))?;
    for row in rows {
        let name: String = row
            .try_get("name")
            .map_err(|e| ArgoError::Provider(format!("memory table_info name: {e}")))?;
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}

async fn exec_schema(pool: &sqlx::Pool<sqlx::Sqlite>, sql: &str) -> ArgoResult<()> {
    sqlx::query(sql)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| ArgoError::Provider(format!("memory layer schema: {e}")))
}

fn fts_query(query: &str) -> Option<String> {
    let mut terms = Vec::new();
    for term in query.split(|c: char| !c.is_ascii_alphanumeric()) {
        if term.len() < 2 {
            continue;
        }
        let lowered = term.to_ascii_lowercase();
        if !terms.contains(&lowered) {
            terms.push(lowered);
        }
        if terms.len() >= 8 {
            break;
        }
    }
    if terms.is_empty() {
        None
    } else {
        Some(
            terms
                .into_iter()
                .map(|term| format!("{term}*"))
                .collect::<Vec<_>>()
                .join(" OR "),
        )
    }
}
