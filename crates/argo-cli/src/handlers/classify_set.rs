//! `argo classify set` handler — insert a `data_classification_tag`.

use argo_identity::DataClass;
use argo_store::classification::{insert_tag, micros_to_rfc3339};
use argo_types::{ArgoError, ArgoResult};
use std::str::FromStr;

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;

pub async fn run(
    target: &str,
    class: &str,
    tagged_by: Option<&str>,
    json: bool,
) -> ArgoResult<()> {
    let (kind, id) = parse_target(target)?;
    let parsed_class = DataClass::from_str(class)
        .map_err(|e| ArgoError::Provider(format!("class: {e}")))?;
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let tagged_at = insert_tag(store.pool(), kind, id, parsed_class.as_str(), tagged_by).await?;
    if json {
        let line = serde_json::to_string(&serde_json::json!({
            "ok": true,
            "record_kind": kind,
            "record_id": id,
            "class": parsed_class.as_str(),
            "tagged_at_micros": tagged_at,
            "tagged_at": micros_to_rfc3339(tagged_at),
            "tagged_by": tagged_by,
        }))
        .unwrap_or_default();
        println!("{line}");
    } else {
        println!(
            "tagged {kind}:{id} as {} ({})",
            parsed_class.as_str(),
            micros_to_rfc3339(tagged_at)
        );
    }
    Ok(())
}

fn parse_target(target: &str) -> ArgoResult<(&str, &str)> {
    let (kind, id) = target.split_once(':').ok_or_else(|| {
        ArgoError::Provider(format!(
            "expected '<kind>:<id>', got '{target}' (e.g. 'memory_entry:abc')"
        ))
    })?;
    if kind.is_empty() || id.is_empty() {
        return Err(ArgoError::Provider(format!(
            "kind and id must both be non-empty (got '{target}')"
        )));
    }
    Ok((kind, id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_target_splits_on_colon() {
        let (k, i) = parse_target("memory_entry:abc-123").unwrap();
        assert_eq!(k, "memory_entry");
        assert_eq!(i, "abc-123");
    }

    #[test]
    fn parse_target_rejects_missing_colon() {
        let err = parse_target("memory_entry").unwrap_err();
        assert!(format!("{err}").contains("expected"));
    }

    #[test]
    fn parse_target_rejects_empty_halves() {
        assert!(parse_target(":abc").is_err());
        assert!(parse_target("memory_entry:").is_err());
    }
}
