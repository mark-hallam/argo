//! Unit tests for [`super::ImageResolver`].
use super::ImageResolver;
use std::collections::BTreeMap;

#[test]
fn empty_resolver_returns_none() {
    let resolver = ImageResolver::default();
    assert!(resolver.is_empty());
    assert_eq!(resolver.len(), 0);
    assert_eq!(resolver.image_for_executor("browser"), None);
}

#[test]
fn resolver_returns_configured_image() {
    let mut map = BTreeMap::new();
    map.insert(
        "browser".to_string(),
        "skycot/marketplace-browser:latest".to_string(),
    );
    let resolver = ImageResolver::from_map(map);
    assert!(!resolver.is_empty());
    assert_eq!(resolver.len(), 1);
    assert_eq!(
        resolver.image_for_executor("browser"),
        Some("skycot/marketplace-browser:latest"),
    );
}

#[test]
fn unknown_executor_returns_none_even_when_map_populated() {
    let mut map = BTreeMap::new();
    map.insert("browser".to_string(), "img:v1".to_string());
    map.insert("code_runner".to_string(), "img:v2".to_string());
    let resolver = ImageResolver::from_map(map);
    assert_eq!(resolver.image_for_executor("nonexistent"), None);
    // Existing entries still resolve.
    assert_eq!(resolver.image_for_executor("browser"), Some("img:v1"));
    assert_eq!(resolver.image_for_executor("code_runner"), Some("img:v2"));
}

#[test]
fn resolver_clones_independently_of_source_map() {
    let mut map = BTreeMap::new();
    map.insert("browser".to_string(), "img:v1".to_string());
    let resolver = ImageResolver::from_map(map.clone());
    // Mutating the source map after `from_map` must NOT affect the
    // resolver — `from_map` takes ownership.
    map.insert("browser".to_string(), "img:v2".to_string());
    assert_eq!(resolver.image_for_executor("browser"), Some("img:v1"));
}
