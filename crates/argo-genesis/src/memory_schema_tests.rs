use super::*;
use crate::archetype::Archetype;

#[test]
fn solo_role_gets_worker_namespace() {
    let schema = generate(
        "draft customer replies",
        Some(&Archetype::CustomerSupportAgent),
    )
    .unwrap();
    assert_eq!(schema.namespaces.len(), 1);
    assert_eq!(schema.namespaces[0].scope, NamespaceScope::Worker);
}

#[test]
fn team_keyword_adds_team_namespace() {
    let schema = generate("coordinate team handoff notes", None).unwrap();
    let scopes: Vec<NamespaceScope> = schema.namespaces.iter().map(|ns| ns.scope).collect();
    assert_eq!(scopes, vec![NamespaceScope::Worker, NamespaceScope::Team]);
}

#[test]
fn policy_keyword_adds_worker_team_and_org() {
    let schema = generate("apply company policy and compliance rules", None).unwrap();
    let scopes: Vec<NamespaceScope> = schema.namespaces.iter().map(|ns| ns.scope).collect();
    assert_eq!(
        scopes,
        vec![
            NamespaceScope::Worker,
            NamespaceScope::Team,
            NamespaceScope::Org
        ]
    );
}

#[test]
fn layers_always_have_retrieval_policy() {
    let schema = generate("research accounts", Some(&Archetype::SalesSdr)).unwrap();
    assert!(!schema.layers.is_empty());
    assert!(schema
        .layers
        .iter()
        .all(|layer| !layer.retrieval_policy.is_empty()));
}

#[test]
fn to_toml_round_trips_as_toml_value() {
    let schema = generate("shared team compliance policy", None).unwrap();
    let body = to_toml(&schema);
    let value: toml::Value = toml::from_str(&body).unwrap();
    assert_eq!(value["schema_version"].as_str(), Some("1"));
    assert_eq!(value["entry"].as_array().map(Vec::len), Some(3));
}

#[test]
fn generated_toml_matches_vertical_parser_shape() {
    let schema = generate("company policy role", None).unwrap();
    let body = to_toml(&schema);
    argo_vertical::parse_memory_schema(&body).unwrap();
}
