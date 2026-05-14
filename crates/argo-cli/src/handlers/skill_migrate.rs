//! `argo skill migrate <id> --from <v> --to <v> [--auto-upgrade]
//! [--reason "..."]` — publisher-side recorder for the
//! `skill_version_migration` table
//! (`mission-skill-version-migration-path`).
//!
//! Records a single row declaring "v1 -> v2 is an upgrade path";
//! tenant-side machinery (`argo skill upgrade`, agent session-boot
//! hint) reads it. Idempotent on the composite PK so re-running a
//! publish pipeline twice is safe.
//!
//! Sensible deviation: the brief calls the positional argument a
//! `<skill_id>`. We accept both the 32-char hex form (as printed by
//! `argo skill versions --json`) AND the dashed-UUID form to match
//! how operators copy ids out of other CLIs.

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_skill::{parse_skill_id_hex, record_migration};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

/// Run `argo skill migrate`. The publisher_tenant_id defaults to a
/// fresh per-call id when no operator supplies one — the brief's
/// "publisher-tenant-matches" enforcement is documented as a TODO
/// (skill ownership belongs to a separate brief).
pub async fn run(
    skill_id_str: &str,
    from_version: &str,
    to_version: &str,
    auto_upgrade: bool,
    reason: Option<&str>,
    publisher_tenant_id_str: Option<&str>,
    json_out: bool,
) -> ArgoResult<()> {
    if from_version == to_version {
        return Err(ArgoError::Schema(
            "--from and --to must differ".to_string(),
        ));
    }
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let db = db_path(&home);
    let store = Arc::new(SessionStore::open(&db).await?);
    store.migrate().await?;

    let skill_id = parse_skill_id_hex(skill_id_str)?;
    let publisher_tenant_id = match publisher_tenant_id_str {
        Some(s) => {
            let parsed = Uuid::parse_str(s).map_err(|e| {
                ArgoError::Schema(format!("publisher tenant id parse: {e}"))
            })?;
            TenantId(parsed)
        }
        None => TenantId::new(),
    };

    record_migration(
        &store,
        skill_id,
        from_version,
        to_version,
        publisher_tenant_id,
        reason,
        auto_upgrade,
    )
    .await?;

    print_result(
        json_out,
        skill_id_str,
        from_version,
        to_version,
        auto_upgrade,
        reason,
    );
    Ok(())
}

fn print_result(
    json_out: bool,
    skill_id: &str,
    from_version: &str,
    to_version: &str,
    auto_upgrade: bool,
    reason: Option<&str>,
) {
    if json_out {
        let payload = json!({
            "ok": true,
            "skill_id": skill_id,
            "from_version": from_version,
            "to_version": to_version,
            "auto_upgrade": auto_upgrade,
            "reason": reason,
        });
        println!("{}", serde_json::to_string(&payload).unwrap_or_default());
    } else {
        let auto = if auto_upgrade { " (auto)" } else { "" };
        println!(
            "recorded migration {skill_id}: {from_version} -> {to_version}{auto}"
        );
        if let Some(r) = reason {
            println!("  reason: {r}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_skill::{list_for_skill, MigrationRow};
    use argo_types::SkillId;

    async fn open_store() -> SessionStore {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        store
    }

    #[tokio::test]
    async fn rejects_same_from_and_to() {
        let id = SkillId::new();
        let hex: String = id.0.as_bytes().iter().map(|b| format!("{b:02x}")).collect();
        let err = run(&hex, "v1", "v1", false, None, None, true)
            .await
            .unwrap_err();
        match err {
            ArgoError::Schema(msg) => assert!(msg.contains("must differ")),
            other => panic!("expected Schema, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn accepts_dashed_uuid_form() {
        let id = SkillId::new();
        let dashed = id.0.to_string();
        let parsed = parse_skill_id_hex(&dashed).unwrap();
        assert_eq!(parsed, id);
    }

    #[tokio::test]
    async fn record_and_list_roundtrip() {
        let store = open_store().await;
        let skill_id = SkillId::new();
        let publisher = TenantId::new();
        record_migration(&store, skill_id, "v1", "v2", publisher, Some("hi"), true)
            .await
            .unwrap();
        let rows: Vec<MigrationRow> = list_for_skill(&store, skill_id).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].from_version, "v1");
        assert!(rows[0].auto_upgrade);
    }
}
