//! `argo skill list-migrations [--json]` — operator dashboard
//! that lists every pending migration across the tenant's
//! installed skills (`mission-skill-version-migration-path`).
//!
//! For each active skill in the workspace, runs
//! [`argo_skill::lookup_migration`] against its installed version
//! and collects the rows whose `from_version` matches. Rows with no
//! pending migration are omitted from the output.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_skill::{lookup_migration, MigrationRow, SkillStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use serde_json::json;
use std::sync::Arc;

/// Run `argo skill list-migrations`.
pub async fn run(json_out: bool) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let db = db_path(&home);
    let store = Arc::new(SessionStore::open(&db).await?);
    store.migrate().await?;
    let skills = SkillStore::new(Arc::clone(&store));
    let pending = collect_pending(&store, &skills).await?;
    if json_out {
        let payload = json!({
            "ok": true,
            "count": pending.len(),
            "migrations": pending.iter().map(|(name, m)| {
                json!({
                    "skill": name,
                    "installed_version": m.from_version,
                    "to_version": m.to_version,
                    "auto_upgrade": m.auto_upgrade,
                    "reason": m.reason,
                })
            }).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string(&payload).unwrap_or_default());
    } else if pending.is_empty() {
        println!("no pending migrations");
    } else {
        for (name, m) in &pending {
            let auto = if m.auto_upgrade { " (auto)" } else { "" };
            println!(
                "{name}: v{} -> v{}{auto}",
                m.from_version, m.to_version
            );
        }
    }
    Ok(())
}

async fn collect_pending(
    store: &SessionStore,
    skills: &SkillStore,
) -> ArgoResult<Vec<(String, MigrationRow)>> {
    let active = skills.list_active().await?;
    let mut out = Vec::new();
    for s in active {
        if let Some(m) = lookup_migration(store, s.id, &s.frontmatter.version).await? {
            out.push((s.frontmatter.name, m));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_skill::record_migration;
    use argo_types::TenantId;

    async fn open_store() -> SessionStore {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        store
    }

    fn skill_md(name: &str, version: &str) -> String {
        format!(
            "---\nname: {name}\nversion: \"{version}\"\ndescription: x\n\
             provenance:\n  source: builtin\n  hash: sha256-x\n---\nbody"
        )
    }

    /// CLI-shape unit: `collect_pending` surfaces only skills with
    /// a matching `from_version` row.
    #[tokio::test]
    async fn collect_pending_filters_to_matching_from() {
        let store = Arc::new(open_store().await);
        let skills = SkillStore::new(Arc::clone(&store));
        let installed = skills.install(&skill_md("a", "1"), None).await.unwrap();
        let _other = skills.install(&skill_md("b", "1"), None).await.unwrap();
        record_migration(&store, installed.id, "1", "2", TenantId::new(), None, false)
            .await
            .unwrap();
        let pending = collect_pending(&store, &skills).await.unwrap();
        assert_eq!(pending.len(), 1, "only `a` has a pending migration");
        assert_eq!(pending[0].0, "a");
        assert_eq!(pending[0].1.to_version, "2");
    }

    /// Integration smoke: record a migration on a freshly-installed
    /// skill, look it up via the same path the upgrade --dry-run
    /// CLI uses, and confirm the published row surfaces. Exercises
    /// the full chain: schema → record_migration → lookup_migration
    /// → collect_pending.
    #[tokio::test]
    async fn integration_migrate_then_dry_run_lookup() {
        let store = Arc::new(open_store().await);
        let skills = SkillStore::new(Arc::clone(&store));
        let installed = skills.install(&skill_md("greet", "1.0"), None).await.unwrap();
        record_migration(
            &store,
            installed.id,
            "1.0",
            "2.0",
            TenantId::new(),
            Some("v2 fixes the trailing-slash bug"),
            false,
        )
        .await
        .unwrap();

        // Mirrors `argo skill upgrade greet --dry-run`: lookup the
        // active version's pending migration row.
        let active = skills.find_active_by_name("greet").await.unwrap().unwrap();
        let migration = lookup_migration(&store, active.id, &active.frontmatter.version)
            .await
            .unwrap()
            .expect("expected a pending migration row after migrate");
        assert_eq!(migration.to_version, "2.0");
        assert!(!migration.auto_upgrade, "auto_upgrade=false must round-trip");

        // Mirrors `argo skill list-migrations`: collect across all
        // active skills.
        let pending = collect_pending(&store, &skills).await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].0, "greet");
    }
}
