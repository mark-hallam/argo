use super::list_response_from_dir;
use serde_json::Value;
use std::path::Path;

const MIN_TOML: &str = r#"schema_version = "1"

[vertical]
id = "{id}"
name = "{name}"
version = "0.1.0"
description = "{desc}"
tags = ["{id}"]
"#;

fn write_vertical(root: &Path, id: &str, name: &str, desc: &str) {
    let dir = root.join(id);
    std::fs::create_dir_all(&dir).unwrap();
    let toml = MIN_TOML
        .replace("{id}", id)
        .replace("{name}", name)
        .replace("{desc}", desc);
    std::fs::write(dir.join("vertical.toml"), toml).unwrap();
}

fn parse(body: &str) -> Value {
    serde_json::from_str(body).expect("json body")
}

#[test]
fn catalogue_returns_packs_loaded_from_disk() {
    let tmp = tempfile::tempdir().unwrap();
    write_vertical(tmp.path(), "demo-a", "Demo A", "Helps with A tasks.");
    write_vertical(tmp.path(), "demo-b", "Demo B", "Helps with B tasks.");

    let (status, body) = list_response_from_dir(tmp.path());

    assert_eq!(status, "200 OK");
    let parsed = parse(&body);
    let rows = parsed["verticals"].as_array().unwrap();
    assert_eq!(rows.len(), 2, "body={body}");
    let ids: Vec<&str> = rows.iter().map(|r| r["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&"demo-a"), "ids={ids:?}");
    assert!(ids.contains(&"demo-b"), "ids={ids:?}");
}

#[test]
fn catalogue_row_has_spa_required_fields() {
    let tmp = tempfile::tempdir().unwrap();
    write_vertical(tmp.path(), "card", "Card Pack", "One sentence summary. Long tail.");

    let (status, body) = list_response_from_dir(tmp.path());

    assert_eq!(status, "200 OK");
    let parsed = parse(&body);
    let row = &parsed["verticals"][0];
    assert_eq!(row["id"], "card");
    assert_eq!(row["name"], "Card Pack");
    assert_eq!(row["version"], "0.1.0");
    assert_eq!(row["headline"], "One sentence summary.");
    assert_eq!(row["role_label"], "Card Pack");
    assert_eq!(row["workflow_count"], 0);
    assert_eq!(row["action_count"], 0);
    assert_eq!(row["skill_count"], 0);
    assert_eq!(row["kpi_count"], 0);
    assert!(row["tags"].is_array());
}

#[test]
fn empty_dir_returns_empty_array_not_tenant_scoped_error() {
    // The previous tenant-scoped implementation returned 500 when
    // no `SessionStore` was passed in. Catalogue semantics: an empty
    // directory yields `{"verticals": []}` — never a 5xx, never a
    // store-unavailable envelope.
    let tmp = tempfile::tempdir().unwrap();
    let (status, body) = list_response_from_dir(tmp.path());
    assert_eq!(status, "200 OK");
    let parsed = parse(&body);
    assert_eq!(parsed["verticals"].as_array().unwrap().len(), 0);
}
