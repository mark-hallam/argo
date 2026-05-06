//! Vertical hard-delete support for `argo vertical`.

use super::vertical_archive::{
    load_installed, normalize_id, open_store, print_json, vertical_archived_at,
};
use argo_types::{ArgoError, ArgoResult};
use chrono::Utc;
use serde::Serialize;
use serde_json::json;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
struct VerticalDeleteOut {
    id: String,
    deleted_path: String,
    deleted_at: i64,
    archived_at: i64,
}

pub async fn run(id: &str, force: bool, json_out: bool) -> ArgoResult<()> {
    let id = normalize_id(id)?;
    let loaded = load_installed(id)?;
    let (store, _) = open_store().await?;
    let out = delete_vertical(&store, id, loaded.root, force).await?;
    if json_out {
        print_json(&json!({"ok": true, "vertical_delete": out}))
    } else {
        println!("deleted vertical {} from {}", out.id, out.deleted_path);
        Ok(())
    }
}

async fn delete_vertical(
    store: &argo_store::SessionStore,
    id: &str,
    root: PathBuf,
    force: bool,
) -> ArgoResult<VerticalDeleteOut> {
    if !force {
        return Err(ArgoError::Provider(
            "vertical delete: pass --force to permanently delete".into(),
        ));
    }
    let archived_at = vertical_archived_at(store, id).await?.ok_or_else(|| {
        ArgoError::Provider("vertical delete: archive vertical before delete --force".into())
    })?;
    std::fs::remove_dir_all(&root)
        .map_err(|e| ArgoError::Provider(format!("vertical delete {}: {e}", root.display())))?;
    Ok(VerticalDeleteOut {
        id: id.to_string(),
        deleted_path: root.display().to_string(),
        deleted_at: Utc::now().timestamp_micros(),
        archived_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::vertical_archive::{archive_vertical, ensure_vertical_lifecycle_schema};
    use argo_identity::IdentityStore;
    use argo_store::SessionStore;

    async fn store() -> SessionStore {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        IdentityStore::new(&store).ensure_schema().await.unwrap();
        ensure_vertical_lifecycle_schema(&store).await.unwrap();
        store
    }

    #[tokio::test]
    async fn delete_force_rejects_non_archived_vertical() {
        let store = store().await;
        let root = tempfile::tempdir().unwrap();
        let err = delete_vertical(&store, "demo", root.path().to_path_buf(), true)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("archive vertical before delete"));
    }

    #[tokio::test]
    async fn delete_requires_force() {
        let store = store().await;
        let root = tempfile::tempdir().unwrap();
        archive_vertical(&store, "demo", 42).await.unwrap();
        let err = delete_vertical(&store, "demo", root.path().to_path_buf(), false)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("--force"));
    }

    #[tokio::test]
    async fn delete_force_removes_archived_directory() {
        let store = store().await;
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("demo");
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join("vertical.toml"), "schema_version = \"1\"\n").unwrap();
        archive_vertical(&store, "demo", 42).await.unwrap();

        let out = delete_vertical(&store, "demo", path.clone(), true)
            .await
            .unwrap();

        assert_eq!(out.id, "demo");
        assert_eq!(out.archived_at, 42);
        assert!(!path.exists());
    }
}
