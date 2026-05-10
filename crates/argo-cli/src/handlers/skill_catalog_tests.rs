//! Unit tests for the pure pieces of `argo skill catalog`. The
//! filter pipeline and JSON envelope are exercised against
//! hand-rolled `MarketplaceSkill` fixtures so the tests don't need a
//! real `SkillStore`. The full end-to-end CLI invocation is covered
//! in `tests/skill_catalog_smoke.rs`.

use super::{apply_filters, entry_to_json, CatalogFilters};
use argo_skill::marketplace::MarketplaceSkill;
use argo_skill::TrustGrade;
use std::collections::HashMap;

fn mk(name: &str, tenant: &str, description: &str, version: &str) -> MarketplaceSkill {
    MarketplaceSkill {
        name: name.into(),
        tenant: tenant.into(),
        description: description.into(),
        version: version.into(),
        trust_grade: TrustGrade::PublishedVerified,
        last_published_at: "2026-05-04T10:15:00Z".into(),
        oci_ref: format!("ghcr.io/skycot/skills/{name}:{version}"),
    }
}

fn sample_market() -> Vec<MarketplaceSkill> {
    vec![
        mk("alpha", "acme", "Drafts buyer-safe alpha listings.", "1.0.0"),
        mk(
            "beta",
            "globex",
            "Summarizes support tickets fast.",
            "2.0.0",
        ),
        mk(
            "gamma",
            "acme",
            "Generates marketplace listing photos.",
            "0.3.0",
        ),
    ]
}

#[test]
fn empty_marketplace_yields_empty_entries() {
    let installed: HashMap<String, String> = HashMap::new();
    let entries = apply_filters(&[], &installed, &CatalogFilters::default());
    assert!(entries.is_empty());
}

#[test]
fn entry_marked_installed_when_local_has_matching_name() {
    let market = sample_market();
    let mut installed = HashMap::new();
    installed.insert("alpha".to_string(), "0.9.0".to_string());
    let entries = apply_filters(&market, &installed, &CatalogFilters::default());
    assert_eq!(entries.len(), 3);
    let alpha = entries.iter().find(|e| e.id == "alpha").unwrap();
    assert!(alpha.installed);
    assert_eq!(alpha.installed_version.as_deref(), Some("0.9.0"));
    let beta = entries.iter().find(|e| e.id == "beta").unwrap();
    assert!(!beta.installed);
    assert!(beta.installed_version.is_none());
}

#[test]
fn vertical_filter_applies() {
    let market = sample_market();
    let installed = HashMap::new();
    let filters = CatalogFilters {
        vertical: Some("acme"),
        ..Default::default()
    };
    let entries = apply_filters(&market, &installed, &filters);
    assert_eq!(entries.len(), 2);
    assert!(entries.iter().all(|e| e.vertical == "acme"));
}

#[test]
fn tag_filter_substring_matches_description() {
    let market = sample_market();
    let installed = HashMap::new();
    let filters = CatalogFilters {
        tag: Some("listing"),
        ..Default::default()
    };
    let entries = apply_filters(&market, &installed, &filters);
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"alpha"));
    assert!(names.contains(&"gamma"));
    assert!(!names.contains(&"beta"));
}

#[test]
fn installed_only_filters_out_uninstalled() {
    let market = sample_market();
    let mut installed = HashMap::new();
    installed.insert("alpha".to_string(), "0.9.0".to_string());
    let filters = CatalogFilters {
        installed_only: true,
        ..Default::default()
    };
    let entries = apply_filters(&market, &installed, &filters);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, "alpha");
    assert!(entries[0].installed);
}

#[test]
fn not_installed_filters_out_installed() {
    let market = sample_market();
    let mut installed = HashMap::new();
    installed.insert("alpha".to_string(), "0.9.0".to_string());
    let filters = CatalogFilters {
        not_installed: true,
        ..Default::default()
    };
    let entries = apply_filters(&market, &installed, &filters);
    assert_eq!(entries.len(), 2);
    assert!(entries.iter().all(|e| !e.installed));
    assert!(entries.iter().any(|e| e.id == "beta"));
    assert!(entries.iter().any(|e| e.id == "gamma"));
}

#[test]
fn json_envelope_shape_is_stable() {
    let market = sample_market();
    let mut installed = HashMap::new();
    installed.insert("alpha".to_string(), "0.9.0".to_string());
    let entries = apply_filters(&market, &installed, &CatalogFilters::default());
    let alpha = entries.iter().find(|e| e.id == "alpha").unwrap();
    let v = entry_to_json(alpha);
    let obj = v.as_object().unwrap();
    // Required keys present for an installed entry.
    for key in [
        "id",
        "name",
        "vertical",
        "tags",
        "installed",
        "version",
        "installed_version",
    ] {
        assert!(obj.contains_key(key), "missing key: {key}");
    }
    assert_eq!(obj["id"], "alpha");
    assert_eq!(obj["name"], "alpha");
    assert_eq!(obj["vertical"], "acme");
    assert!(obj["tags"].is_array());
    assert_eq!(obj["installed"], true);
    assert_eq!(obj["version"], "1.0.0");
    assert_eq!(obj["installed_version"], "0.9.0");

    // Uninstalled entries omit installed_version.
    let beta = entries.iter().find(|e| e.id == "beta").unwrap();
    let v2 = entry_to_json(beta);
    let obj2 = v2.as_object().unwrap();
    assert_eq!(obj2["installed"], false);
    assert!(!obj2.contains_key("installed_version"));
}
