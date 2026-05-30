//! Demo-seed enrichment for two "theatre" surfaces the camera opens
//! but that read from currently-empty tables:
//!
//! * **`GET /api/v1/skills/<id>/ir`** — the /skills marketplace lists
//!   `skill` (catalog) rows by their UUID; clicking one calls the IR
//!   endpoint, which reads `skill_ir` keyed on the SAME id. The
//!   canonical [`crate::skills`] seed writes `skill_ir` under random
//!   `SkillId::new()` ids disjoint from the catalog, so the click
//!   404s with `"skill has no IR"`. [`seed_skill_ir`] persists a
//!   minimal-but-real [`SkillGraph`] for each catalog skill id via the
//!   real canvas writer ([`save_edited_graph`]) so the hash + shape are
//!   valid and the endpoint returns a real IR.
//!
//! * **`GET /api/v1/workers/<id>/evals`** — a worker's "evals" are the
//!   scheduled re-eval history of its learned skills, read from
//!   `skill_reeval_history` joined on `worker_skill.skill_id`. The
//!   hero workers learn the catalog skills (see
//!   `seed_worker_skill_assignments`), but `skill_reeval_history` is
//!   empty, so the endpoint returns `{latest:null,history:[]}`.
//!   [`seed_skill_reeval_history`] inserts a few real rows per catalog
//!   skill (varied pass rates, run-at spread across past days, one
//!   regressed) so a worker's evals tab shows real trend history.
//!
//! Both surfaces unify on the marketplace `skill` table ids (the ones
//! the SPA lists + the ones the workers learn), so both seeds read the
//! catalog id set from [`catalog_skill_ids`].
//!
//! No migration: `skill_ir` is applied by `argo_skill::ir::ensure_schema`
//! and `skill_reeval_history` is the lazily-applied raw-SQL table at
//! `crates/argo-store/migrations/121__skill_reeval_history.sql`. Both
//! seeds apply their schema defensively before writing.

use std::sync::Arc;

use argo_skill::ir::{
    ensure_schema as ensure_ir_schema, save_edited_graph, Node, NodeAnnotations, NodeId, NodeKind,
    SkillGraph, SkillGraphMetadata,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, SkillId, TenantId};
use chrono::Utc;
use uuid::Uuid;

use crate::SeedResult;

/// Idempotent mirror of `crates/argo-store/migrations/121__skill_reeval_history.sql`.
/// The table is lazily applied by the reeval cron (it is NOT in the
/// migration registry), so a freshly-seeded store would otherwise hit
/// "no such table" here. We apply the same `CREATE TABLE IF NOT EXISTS`
/// defensively before the read, exactly as the gateway's
/// `routes_evals::ensure_reeval_schema` does.
const REEVAL_SCHEMA_SQL: &str =
    include_str!("../../argo-store/migrations/121__skill_reeval_history.sql");

/// Read the demo tenant's active marketplace catalog skill ids as
/// hyphenated UUID strings. These are the ids the /skills SPA lists,
/// the ids `worker_skill` carries after `seed_worker_skill_assignments`,
/// and therefore the ids both theatre surfaces key on. Mirrors the
/// blob → UUID conversion in `seed::seed_worker_skill_assignments`.
async fn catalog_skill_ids(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
) -> SeedResult<Vec<String>> {
    let blobs: Vec<Vec<u8>> = sqlx::query_scalar(
        "SELECT id FROM skill \
         WHERE tenant_id = ? AND lifecycle_state = 'active' AND superseded_by IS NULL \
         ORDER BY name ASC",
    )
    .bind(tenant_id.to_string())
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("catalog_skill_ids: {e}")))?;
    Ok(blobs
        .iter()
        .filter(|b| b.len() == 16)
        .map(|b| {
            let mut a = [0u8; 16];
            a.copy_from_slice(b);
            Uuid::from_bytes(a).to_string()
        })
        .collect())
}

/// A minimal-but-real entry-only [`SkillGraph`] for `name`. A lone
/// `Action` node has outdegree 0, so it is a terminal node and passes
/// both `validate_graph` and `validate_editable_topology` inside
/// [`save_edited_graph`]. The graph is what `GET /skills/<id>/ir`
/// renders back, so the label + instruction read as a real catalog
/// skill rather than a placeholder.
fn entry_only_graph(name: &str) -> SkillGraph {
    let entry: NodeId = "entry".into();
    let node = Node {
        id: entry.clone(),
        label: name.to_string(),
        kind: NodeKind::Action {
            instruction: format!(
                "Run the `{name}` skill end-to-end and emit a normalized result row."
            ),
        },
        annotations: NodeAnnotations::default(),
    };
    SkillGraph {
        entry,
        nodes: vec![node],
        edges: Vec::new(),
        variables: Vec::new(),
        metadata: SkillGraphMetadata {
            name: Some(name.to_string()),
            version: None,
            description: Some(format!("Demo-seed IR for the `{name}` marketplace skill.")),
            source: Some("argo-demo-seed".to_string()),
        },
    }
}

/// Persist a real IR graph for each demo marketplace catalog skill so
/// `GET /api/v1/skills/<id>/ir` returns a valid IR instead of 404. Uses
/// the canonical canvas writer ([`save_edited_graph`]) so the canonical
/// JSON + sha256 hash + hash-chain are all real — not hand-rolled SQL.
///
/// Idempotent: skips any catalog skill that already has IR (the writer
/// would otherwise stack a second `canvas-*` version on every reseed).
/// Returns the number of skills that got a NEW IR graph this run.
pub async fn seed_skill_ir(store: &Arc<SessionStore>, tenant_id: TenantId) -> SeedResult<usize> {
    ensure_ir_schema(store).await?;
    let ids = catalog_skill_ids(store, tenant_id).await?;
    if ids.is_empty() {
        return Ok(0);
    }
    let mut inserted = 0usize;
    for id in &ids {
        let skill_uuid =
            Uuid::parse_str(id).map_err(|e| ArgoError::Provider(format!("skill id parse: {e}")))?;
        let skill_id = SkillId(skill_uuid);
        // Idempotency: if this catalog skill already carries IR, the set
        // was seeded by a prior run — skip so reseeding (e.g. to add a
        // teammate) does not stack a second canvas version.
        if !skill_ir_present(store, tenant_id, skill_id).await? {
            let name = skill_label(store, tenant_id, id).await?;
            let graph = entry_only_graph(&name);
            save_edited_graph(store, tenant_id, skill_id, &graph).await?;
            inserted += 1;
        }
    }
    Ok(inserted)
}

/// True when at least one `skill_ir` row exists for
/// `(tenant_id, skill_id)`.
async fn skill_ir_present(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
    skill_id: SkillId,
) -> SeedResult<bool> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM skill_ir WHERE tenant_id = ? AND skill_id = ?",
    )
    .bind(tenant_id.0.as_bytes().as_slice())
    .bind(skill_id.0.as_bytes().as_slice())
    .fetch_one(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("skill_ir present probe: {e}")))?;
    Ok(count > 0)
}

/// Resolve the catalog `skill.name` for a hyphenated-UUID id under this
/// tenant. Falls back to the id when the row has no name (should not
/// happen for seeded rows). The id is the UUID string; the `skill.id`
/// column is the matching 16-byte blob.
async fn skill_label(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
    id: &str,
) -> SeedResult<String> {
    let uuid =
        Uuid::parse_str(id).map_err(|e| ArgoError::Provider(format!("skill id parse: {e}")))?;
    let name: Option<String> = sqlx::query_scalar(
        "SELECT name FROM skill WHERE tenant_id = ? AND id = ? LIMIT 1",
    )
    .bind(tenant_id.to_string())
    .bind(uuid.as_bytes().as_slice())
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("skill_label: {e}")))?;
    Ok(name.unwrap_or_else(|| id.to_string()))
}

/// Insert scheduled-reeval history rows for the demo marketplace
/// catalog skills so `GET /api/v1/workers/<id>/evals` shows a real
/// trend (the worker's evals = the reeval history of its learned
/// skills). Each catalog skill gets three rows spread across the past
/// fortnight with varied pass rates and one regression, so the SPA's
/// eval tab renders a non-trivial pass/fail trend with a `regressed`
/// decision badge.
///
/// Idempotent: skips the whole block when reeval history already exists
/// for any demo catalog skill (sentinel = the first catalog id), so a
/// reseed does not duplicate the trend. Returns the number of NEW rows
/// inserted this run.
pub async fn seed_skill_reeval_history(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
) -> SeedResult<usize> {
    ensure_reeval_schema(store).await?;
    let ids = catalog_skill_ids(store, tenant_id).await?;
    if ids.is_empty() {
        return Ok(0);
    }
    // Sentinel: any reeval row for the first catalog skill means the set
    // was already seeded — skip the whole block.
    let sentinel = &ids[0];
    let existing: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM skill_reeval_history WHERE skill_id = ?")
            .bind(sentinel)
            .fetch_one(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("reeval sentinel probe: {e}")))?;
    if existing > 0 {
        return Ok(0);
    }

    // (days_ago, pass_rate, previous_pass_rate, regressed) — a three-row
    // arc per skill: an older baseline, a mid run, then the latest. One
    // skill index gets a regressed latest run so the badge renders.
    let now = Utc::now();
    let arc: [(i64, f64, Option<f64>, i64); 3] = [
        (12, 0.86, None, 0),
        (6, 0.91, Some(0.86), 0),
        (1, 0.94, Some(0.91), 0),
    ];
    let regressed_arc: [(i64, f64, Option<f64>, i64); 3] = [
        (11, 0.88, None, 0),
        (5, 0.90, Some(0.88), 0),
        (1, 0.72, Some(0.90), 1),
    ];

    let mut inserted = 0usize;
    for (idx, id) in ids.iter().enumerate() {
        let name = skill_label(store, tenant_id, id).await?;
        // Every third catalog skill shows a regression in its latest run.
        let rows = if idx % 3 == 2 { &regressed_arc } else { &arc };
        for (run_idx, (days_ago, pass_rate, prev, regressed)) in rows.iter().enumerate() {
            let run_at = (now - chrono::Duration::days(*days_ago)).timestamp_micros();
            let eval_run_id = Uuid::now_v7().to_string();
            let version = format!("v{}", run_idx + 1);
            sqlx::query(
                "INSERT INTO skill_reeval_history \
                 (eval_run_id, skill_id, skill_name, skill_version, run_at, \
                  pass_rate, previous_pass_rate, regressed) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&eval_run_id)
            .bind(id)
            .bind(&name)
            .bind(&version)
            .bind(run_at)
            .bind(*pass_rate)
            .bind(*prev)
            .bind(*regressed)
            .execute(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("skill_reeval_history insert: {e}")))?;
            inserted += 1;
        }
    }
    Ok(inserted)
}

/// Apply the lazy `skill_reeval_history` schema (idempotent). Strips SQL
/// comments and runs each statement, matching the bootstrap the gateway
/// eval route + the reeval cron use.
async fn ensure_reeval_schema(store: &Arc<SessionStore>) -> SeedResult<()> {
    let stripped = REEVAL_SCHEMA_SQL
        .lines()
        .filter(|line| !line.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n");
    for stmt in stripped.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        sqlx::query(stmt)
            .execute(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("ensure skill_reeval_history: {e}")))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::marketplace::seed_demo_marketplace_skills;
    use crate::{demo_marketplace_catalog_size, SeedOptions};

    async fn open_store() -> Arc<SessionStore> {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        Arc::new(store)
    }

    #[tokio::test]
    async fn seeds_ir_for_every_catalog_skill() {
        let store = open_store().await;
        let tenant = TenantId::new();
        seed_demo_marketplace_skills(&store, tenant).await.unwrap();
        let inserted = seed_skill_ir(&store, tenant).await.unwrap();
        assert_eq!(
            inserted,
            demo_marketplace_catalog_size(),
            "one IR graph per catalog skill"
        );
        // Ground truth: each catalog skill id now has a skill_ir row that
        // GET /skills/<id>/ir reads back.
        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM skill_ir WHERE tenant_id = ?")
            .bind(tenant.0.as_bytes().as_slice())
            .fetch_one(store.pool())
            .await
            .unwrap();
        assert_eq!(rows, demo_marketplace_catalog_size() as i64);
    }

    #[tokio::test]
    async fn reseed_ir_is_noop() {
        let store = open_store().await;
        let tenant = TenantId::new();
        seed_demo_marketplace_skills(&store, tenant).await.unwrap();
        let first = seed_skill_ir(&store, tenant).await.unwrap();
        let second = seed_skill_ir(&store, tenant).await.unwrap();
        assert_eq!(first, demo_marketplace_catalog_size());
        assert_eq!(second, 0, "reseed must not stack a second IR version");
        // Exactly one IR version per skill (no canvas-version stacking).
        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM skill_ir WHERE tenant_id = ?")
            .bind(tenant.0.as_bytes().as_slice())
            .fetch_one(store.pool())
            .await
            .unwrap();
        assert_eq!(rows, demo_marketplace_catalog_size() as i64);
    }

    #[tokio::test]
    async fn seeds_reeval_history_with_a_regression() {
        let store = open_store().await;
        let tenant = TenantId::new();
        seed_demo_marketplace_skills(&store, tenant).await.unwrap();
        let inserted = seed_skill_reeval_history(&store, tenant).await.unwrap();
        // Three rows per catalog skill.
        assert_eq!(inserted, demo_marketplace_catalog_size() * 3);
        // At least one regressed row exists so the SPA renders the badge.
        let regressed: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM skill_reeval_history WHERE regressed = 1")
                .fetch_one(store.pool())
                .await
                .unwrap();
        assert!(regressed >= 1, "expected >=1 regressed reeval row");
    }

    #[tokio::test]
    async fn reseed_reeval_history_is_noop() {
        let store = open_store().await;
        let tenant = TenantId::new();
        seed_demo_marketplace_skills(&store, tenant).await.unwrap();
        let first = seed_skill_reeval_history(&store, tenant).await.unwrap();
        let second = seed_skill_reeval_history(&store, tenant).await.unwrap();
        assert_eq!(first, demo_marketplace_catalog_size() * 3);
        assert_eq!(second, 0, "reseed must skip the already-seeded trend");
    }

    #[tokio::test]
    async fn ir_seed_is_tenant_scoped() {
        // Seeding tenant A's IR leaves tenant B with zero IR rows.
        let store = open_store().await;
        let tenant_a = TenantId::new();
        let tenant_b = TenantId::new();
        seed_demo_marketplace_skills(&store, tenant_a).await.unwrap();
        seed_skill_ir(&store, tenant_a).await.unwrap();
        let b_rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM skill_ir WHERE tenant_id = ?")
            .bind(tenant_b.0.as_bytes().as_slice())
            .fetch_one(store.pool())
            .await
            .unwrap();
        assert_eq!(b_rows, 0, "tenant B must see no IR rows");
        // And a tenant with no catalog at all seeds zero (no panic).
        let inserted_b = seed_skill_ir(&store, tenant_b).await.unwrap();
        assert_eq!(inserted_b, 0);
    }

    #[tokio::test]
    async fn full_seed_populates_both_surfaces() {
        // End-to-end through run_canonical_v1_seed: the report fields are
        // > 0 and the worker's learned-skill ids overlap reeval history.
        let store = open_store().await;
        let tenant = TenantId::new();
        let mut opts = SeedOptions::canonical_v1(tenant);
        opts.skip_pass_a_turns = true;
        opts.dressing_audit_rows = 0;
        let report = crate::run_canonical_v1_seed(&store, &opts).await.unwrap();
        assert!(report.skill_ir_seeded > 0, "skill_ir_seeded must be > 0");
        assert!(
            report.skill_reeval_rows_seeded > 0,
            "skill_reeval_rows_seeded must be > 0"
        );

        // The hero workers' learned skills have reeval history: join
        // worker_skill → skill_reeval_history and expect > 0 rows.
        let joined: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM worker_skill ws \
             JOIN worker w ON w.id = ws.worker_id \
             JOIN skill_reeval_history h ON h.skill_id = ws.skill_id \
             WHERE w.tenant_id = ?",
        )
        .bind(tenant.to_string())
        .fetch_one(store.pool())
        .await
        .unwrap();
        assert!(
            joined > 0,
            "a hero worker's learned skill must have reeval history (got {joined})"
        );
    }
}
