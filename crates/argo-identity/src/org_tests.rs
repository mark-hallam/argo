//! Unit tests for `crate::org` + `crate::org_ingest`.
use super::{MemberKind, OrgStore};
use crate::org_ingest::{OrgChartYaml, OrgMemberYaml, OrgUnitYaml};
use argo_store::SessionStore;
use argo_types::TenantId;
use std::sync::Arc;

async fn open_store() -> Arc<OrgStore> {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let org = Arc::new(OrgStore::new(store));
    org.ensure_schema().await.unwrap();
    org
}

fn fixture_chart(tenant: TenantId) -> OrgChartYaml {
    OrgChartYaml {
        tenant: tenant.to_string(),
        org: "Acme".into(),
        units: vec![
            OrgUnitYaml { name: "Engineering".into(), parent: None },
            OrgUnitYaml { name: "Backend".into(), parent: Some("Engineering".into()) },
        ],
        members: vec![
            OrgMemberYaml {
                canonical: "Maria Hernandez".into(),
                kind: MemberKind::Human,
                unit: Some("Engineering".into()),
                manager: None,
                worker_id: None,
                aliases: vec!["Maria".into(), "M.H.".into(), "@maria".into()],
            },
            OrgMemberYaml {
                canonical: "Carlos Rivera".into(),
                kind: MemberKind::Human,
                unit: Some("Backend".into()),
                manager: Some("Maria Hernandez".into()),
                worker_id: None,
                aliases: vec!["Carlos".into()],
            },
            OrgMemberYaml {
                canonical: "skater_bot".into(),
                kind: MemberKind::Worker,
                unit: Some("Backend".into()),
                manager: Some("Carlos Rivera".into()),
                worker_id: Some("worker-uuid-1".into()),
                aliases: vec!["bot".into()],
            },
        ],
    }
}

#[tokio::test]
async fn ingest_writes_units_members_aliases_and_links() {
    let org = open_store().await;
    let tenant = TenantId::new();
    let summary = org.ingest(tenant, &fixture_chart(tenant)).await.unwrap();
    assert_eq!(summary.units, 2);
    assert_eq!(summary.members, 3);
    assert_eq!(summary.aliases, 5); // 3 + 1 + 1
    assert_eq!(summary.report_links, 2); // Carlos → Maria, bot → Carlos
}

#[tokio::test]
async fn list_units_returns_alphabetical() {
    let org = open_store().await;
    let tenant = TenantId::new();
    org.ingest(tenant, &fixture_chart(tenant)).await.unwrap();
    let units = org.list_units(tenant).await.unwrap();
    let names: Vec<&str> = units.iter().map(|u| u.name.as_str()).collect();
    assert_eq!(names, vec!["Backend", "Engineering"]);
}

#[tokio::test]
async fn list_members_includes_aliases() {
    let org = open_store().await;
    let tenant = TenantId::new();
    org.ingest(tenant, &fixture_chart(tenant)).await.unwrap();
    let members = org.list_members(tenant).await.unwrap();
    let maria = members
        .iter()
        .find(|m| m.canonical == "Maria Hernandez")
        .unwrap();
    assert!(maria.aliases.contains(&"Maria".to_string()));
    assert!(maria.aliases.contains(&"@maria".to_string()));
}

#[tokio::test]
async fn resolve_canonical_returns_full_member() {
    let org = open_store().await;
    let tenant = TenantId::new();
    org.ingest(tenant, &fixture_chart(tenant)).await.unwrap();
    let m = org.resolve(tenant, "Maria Hernandez").await.unwrap().unwrap();
    assert_eq!(m.canonical, "Maria Hernandez");
    assert_eq!(m.kind, MemberKind::Human);
}

#[tokio::test]
async fn resolve_alias_falls_through_to_canonical() {
    let org = open_store().await;
    let tenant = TenantId::new();
    org.ingest(tenant, &fixture_chart(tenant)).await.unwrap();
    let m = org.resolve(tenant, "@maria").await.unwrap().unwrap();
    assert_eq!(m.canonical, "Maria Hernandez");
}

#[tokio::test]
async fn resolve_returns_none_for_unknown_name() {
    let org = open_store().await;
    let tenant = TenantId::new();
    org.ingest(tenant, &fixture_chart(tenant)).await.unwrap();
    assert!(org.resolve(tenant, "Ghost").await.unwrap().is_none());
}

#[tokio::test]
async fn reports_to_chain_walks_to_root() {
    let org = open_store().await;
    let tenant = TenantId::new();
    org.ingest(tenant, &fixture_chart(tenant)).await.unwrap();
    let bot = org.resolve(tenant, "skater_bot").await.unwrap().unwrap();
    let chain = org.reports_to_chain(tenant, &bot.id).await.unwrap();
    let names: Vec<&str> = chain.iter().map(|m| m.canonical.as_str()).collect();
    assert_eq!(names, vec!["Carlos Rivera", "Maria Hernandez"]);
}

#[tokio::test]
async fn reports_to_chain_is_empty_for_top_of_org() {
    let org = open_store().await;
    let tenant = TenantId::new();
    org.ingest(tenant, &fixture_chart(tenant)).await.unwrap();
    let maria = org.resolve(tenant, "Maria Hernandez").await.unwrap().unwrap();
    let chain = org.reports_to_chain(tenant, &maria.id).await.unwrap();
    assert!(chain.is_empty());
}

#[tokio::test]
async fn ingest_is_idempotent_on_canonical_pair() {
    let org = open_store().await;
    let tenant = TenantId::new();
    let chart = fixture_chart(tenant);
    let first = org.ingest(tenant, &chart).await.unwrap();
    let second = org.ingest(tenant, &chart).await.unwrap();
    // Second pass adds zero new units / members; aliases hit
    // INSERT OR IGNORE so they're skipped silently.
    let units = org.list_units(tenant).await.unwrap();
    let members = org.list_members(tenant).await.unwrap();
    assert_eq!(units.len(), 2);
    assert_eq!(members.len(), 3);
    assert_eq!(first.units, second.units);
}

#[tokio::test]
async fn ingest_rejects_unknown_parent_unit_reference() {
    let org = open_store().await;
    let tenant = TenantId::new();
    let chart = OrgChartYaml {
        tenant: tenant.to_string(),
        org: "Acme".into(),
        units: vec![OrgUnitYaml {
            name: "Backend".into(),
            parent: Some("NoSuchUnit".into()),
        }],
        members: vec![],
    };
    let err = org.ingest(tenant, &chart).await.unwrap_err();
    assert!(err.to_string().contains("unknown parent"));
}

#[tokio::test]
async fn ingest_rejects_unknown_manager_reference() {
    let org = open_store().await;
    let tenant = TenantId::new();
    let chart = OrgChartYaml {
        tenant: tenant.to_string(),
        org: "Acme".into(),
        units: vec![],
        members: vec![OrgMemberYaml {
            canonical: "Maria".into(),
            kind: MemberKind::Human,
            unit: None,
            manager: Some("NoSuchManager".into()),
            worker_id: None,
            aliases: vec![],
        }],
    };
    let err = org.ingest(tenant, &chart).await.unwrap_err();
    assert!(err.to_string().contains("unknown manager"));
}

#[tokio::test]
async fn yaml_round_trips_cleanly_via_serde_yaml() {
    let tenant = TenantId::new();
    let chart = fixture_chart(tenant);
    let s = serde_yaml::to_string(&chart).unwrap();
    let back: OrgChartYaml = serde_yaml::from_str(&s).unwrap();
    assert_eq!(chart, back);
}

#[tokio::test]
async fn cross_tenant_isolation_prevents_leakage() {
    let org = open_store().await;
    let t1 = TenantId::new();
    let t2 = TenantId::new();
    org.ingest(t1, &fixture_chart(t1)).await.unwrap();
    let units_t1 = org.list_units(t1).await.unwrap();
    let units_t2 = org.list_units(t2).await.unwrap();
    assert_eq!(units_t1.len(), 2);
    assert!(units_t2.is_empty());
}
