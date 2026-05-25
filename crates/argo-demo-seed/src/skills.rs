//! Skill IR catalog inserted by the canonical seed.
//!
//! The seed inserts three rows into the tenant-scoped `skill_ir`
//! table: the Scene-4 headline skill `competitor pricing scrape`
//! plus two siblings that flesh out the marketplace's skill catalog
//! (so the /skills page renders a non-trivial list when the demo
//! opens it). Each skill is a minimal valid [`SkillGraph`] — one
//! `Action` entry node, no edges — because Scene 4 only renders
//! the catalog view, not a runtime invocation of the skill body.
//!
//! Foreign-tenant variant: a single skill is inserted under the
//! foreign tenant id so Scene 6's cross-tenant invisibility beat
//! can query `SELECT COUNT(*) FROM skill_ir WHERE tenant_id = <demo>`
//! and confirm the foreign rows do not show.

use argo_skill::ir::{
    ensure_schema, Node, NodeAnnotations, NodeId, NodeKind, SkillGraph, SkillGraphMetadata,
    SkillIrStore, SqliteSkillIrStore,
};
use argo_store::SessionStore;
use argo_types::{SkillId, TenantId};

use crate::SeedResult;

/// Catalog row used by the seed — the skill name + version + a
/// short body marker that gets dropped into the graph metadata so
/// `argo skill show` produces something more useful than an empty
/// graph.
struct CatalogRow {
    name: &'static str,
    version: &'static str,
    summary: &'static str,
}

/// The Scene-4 canonical skill plus two siblings.
const DEMO_TENANT_SKILLS: &[CatalogRow] = &[
    CatalogRow {
        name: "competitor pricing scrape",
        version: "v1",
        summary: "Scrape competitor product pricing and emit a normalized comparison row.",
    },
    CatalogRow {
        name: "listing photo touch-up",
        version: "v1",
        summary: "Apply the marketplace's brand-safe photo touch-up preset and emit a CDN URL.",
    },
    CatalogRow {
        name: "buyer reply triage",
        version: "v1",
        summary: "Classify a buyer reply (question / haggle / spam) and route to the right action.",
    },
];

/// Scene-6 foreign-tenant skill row.
const FOREIGN_TENANT_SKILL: CatalogRow = CatalogRow {
    name: "competitor pricing scrape",
    version: "v1",
    summary: "[foreign tenant — invisible to demo workspace] competitor pricing scrape sibling.",
};

/// Insert the canonical demo skill catalog under `tenant_id`. Returns
/// the number of `skill_ir` rows inserted.
pub async fn seed_demo_skills(store: &SessionStore, tenant_id: TenantId) -> SeedResult<usize> {
    ensure_schema(store).await?;
    let ir_store = SqliteSkillIrStore::new(store);
    let mut inserted = 0;
    for row in DEMO_TENANT_SKILLS {
        let skill_id = SkillId::new();
        let graph = catalog_graph(row);
        ir_store.put(tenant_id, skill_id, row.version, &graph).await?;
        inserted += 1;
    }
    Ok(inserted)
}

/// Insert the Scene-6 foreign-tenant skill row. Returns the number
/// of rows inserted (always `1` on success).
pub async fn seed_foreign_skill(store: &SessionStore, tenant_id: TenantId) -> SeedResult<usize> {
    ensure_schema(store).await?;
    let ir_store = SqliteSkillIrStore::new(store);
    let skill_id = SkillId::new();
    let graph = catalog_graph(&FOREIGN_TENANT_SKILL);
    ir_store
        .put(tenant_id, skill_id, FOREIGN_TENANT_SKILL.version, &graph)
        .await?;
    Ok(1)
}

fn catalog_graph(row: &CatalogRow) -> SkillGraph {
    let entry: NodeId = format!("entry-{}", row.name.replace(' ', "-")).into();
    let node = Node {
        id: entry.clone(),
        label: row.name.to_string(),
        kind: NodeKind::Action {
            instruction: row.summary.to_string(),
        },
        annotations: NodeAnnotations::default(),
    };
    SkillGraph {
        entry,
        nodes: vec![node],
        edges: Vec::new(),
        variables: Vec::new(),
        metadata: SkillGraphMetadata {
            name: Some(row.name.to_string()),
            version: Some(row.version.to_string()),
            description: Some(row.summary.to_string()),
            source: Some("argo-demo-seed".to_string()),
        },
    }
}
