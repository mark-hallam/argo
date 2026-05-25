#![cfg(test)]

use super::super::{routes::dispatch_route, shutdown::ShutdownController};
use super::{list_response_with_roots, MAX_LIMIT};
use serde_json::Value;
use std::path::Path;

fn pack_toml(name: &str, version: &str, vertical_id: &str) -> String {
    format!(
        r#"[pack]
name = "{name}"
version = "{version}"
author = "Acme"
description = "{name} description"

[[verticals]]
id = "{vertical_id}"
path = "."
"#
    )
}

fn write_pack(root: &Path, name: &str, version: &str, vertical_id: &str) {
    std::fs::create_dir_all(root).expect("pack root");
    std::fs::write(
        root.join("pack.toml"),
        pack_toml(name, version, vertical_id),
    )
    .expect("pack.toml");
}

fn parse_ok(req: &str, repo: &Path, cache: &Path) -> Value {
    let (status, body) = list_response_with_roots(req, repo, cache);
    assert_eq!(status, "200 OK", "body: {body}");
    serde_json::from_str(&body).expect("json")
}

fn reset_auth_env() {
    for key in [
        "ARGO_GATEWAY_BEARER_TOKEN",
        "ARGO_GATEWAY_AUTH_BYPASS",
        "ARGO_GATEWAY_PROFILE",
    ] {
        unsafe { std::env::remove_var(key) };
    }
}

#[test]
fn empty_catalog_returns_stable_paginated_shape() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let repo = tmp.path().join("repo-packs-missing");
    let cache = tmp.path().join("cache");
    let value = parse_ok("GET /api/v1/packs HTTP/1.1\r\n\r\n", &repo, &cache);

    assert_eq!(value["packs"].as_array().expect("packs").len(), 0);
    assert_eq!(value["pagination"]["total"], 0);
    assert_eq!(value["pagination"]["limit"], 50);
    assert_eq!(value["pagination"]["offset"], 0);
    assert_eq!(value["pagination"]["has_more"], false);
}

#[test]
fn repo_pack_catalog_row_has_spa_contract_fields() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let repo = tmp.path().join("repo-packs");
    let cache = tmp.path().join("cache");
    write_pack(
        &repo.join("ap-specialist"),
        "ap-specialist",
        "1.0.0",
        "ap-specialist",
    );

    let value = parse_ok("GET /api/v1/packs?limit=1 HTTP/1.1\r\n\r\n", &repo, &cache);
    let packs = value["packs"].as_array().expect("packs");
    assert_eq!(packs.len(), 1);
    assert_eq!(packs[0]["id"], "ap-specialist");
    assert_eq!(packs[0]["version"], "1.0.0");
    assert_eq!(packs[0]["vertical_id"], "ap-specialist");
    assert_eq!(packs[0]["author"], "Acme");
    assert_eq!(packs[0]["description"], "ap-specialist description");
    assert_eq!(packs[0]["installed"], false);
    assert_eq!(packs[0]["source"], "repo");
}

#[test]
fn cache_metadata_marks_repo_pack_installed_and_adds_cache_only_rows() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let repo = tmp.path().join("repo-packs");
    let cache = tmp.path().join("cache");
    write_pack(&repo.join("recruiter"), "recruiter", "1.0.0", "recruiter");
    write_pack(
        &cache.join("recruiter").join("1.0.0"),
        "recruiter",
        "1.0.0",
        "recruiter",
    );
    write_pack(
        &cache.join("custom").join("0.2.0"),
        "custom",
        "0.2.0",
        "custom-vertical",
    );

    let value = parse_ok("GET /api/v1/packs?limit=10 HTTP/1.1\r\n\r\n", &repo, &cache);
    let packs = value["packs"].as_array().expect("packs");
    assert_eq!(packs.len(), 2);
    let custom = packs.iter().find(|p| p["id"] == "custom").expect("custom");
    assert_eq!(custom["installed"], true);
    assert_eq!(custom["source"], "cache");
    let recruiter = packs
        .iter()
        .find(|p| p["id"] == "recruiter")
        .expect("recruiter");
    assert_eq!(recruiter["installed"], true);
    assert_eq!(recruiter["source"], "repo");
}

#[test]
fn pagination_offset_and_limit_are_stable_and_capped() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let repo = tmp.path().join("repo-packs");
    let cache = tmp.path().join("cache");
    write_pack(&repo.join("a"), "a", "1.0.0", "a");
    write_pack(&repo.join("b"), "b", "1.0.0", "b");
    write_pack(&repo.join("c"), "c", "1.0.0", "c");

    let value = parse_ok(
        "GET /api/v1/packs?limit=2&offset=1 HTTP/1.1\r\n\r\n",
        &repo,
        &cache,
    );
    let ids: Vec<&str> = value["packs"]
        .as_array()
        .expect("packs")
        .iter()
        .filter_map(|p| p["id"].as_str())
        .collect();
    assert_eq!(ids, vec!["b", "c"]);
    assert_eq!(value["pagination"]["total"], 3);
    assert_eq!(value["pagination"]["has_more"], false);

    let capped = parse_ok(
        "GET /api/v1/packs?limit=999 HTTP/1.1\r\n\r\n",
        &repo,
        &cache,
    );
    assert_eq!(capped["pagination"]["limit"], MAX_LIMIT);
}

#[test]
fn invalid_pagination_returns_400() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (status, body) = list_response_with_roots(
        "GET /api/v1/packs?limit=nope HTTP/1.1\r\n\r\n",
        &tmp.path().join("repo"),
        &tmp.path().join("cache"),
    );
    assert_eq!(status, "400 Bad Request");
    assert!(body.contains("invalid limit"));
}

#[tokio::test]
async fn api_route_is_bearer_gated() {
    reset_auth_env();
    let home = tempfile::tempdir().expect("tempdir");
    let old_home = std::env::var_os("HOME");
    unsafe {
        std::env::set_var("ARGO_GATEWAY_BEARER_TOKEN", "secret");
        std::env::set_var("HOME", home.path());
    }
    let shutdown = ShutdownController::new();

    let (status, _) = dispatch_route(
        "GET /api/v1/packs HTTP/1.1\r\nHost: x\r\n\r\n",
        None,
        &shutdown,
    )
    .await;
    assert_eq!(status, "401 Unauthorized");

    let (status, body) = dispatch_route(
        "GET /api/v1/packs?limit=1 HTTP/1.1\r\nHost: x\r\nAuthorization: Bearer secret\r\n\r\n",
        None,
        &shutdown,
    )
    .await;
    assert_eq!(status, "200 OK", "body: {body}");
    reset_auth_env();
    unsafe {
        if let Some(old_home) = old_home {
            std::env::set_var("HOME", old_home);
        } else {
            std::env::remove_var("HOME");
        }
    }
}
