//! Unit tests for the marketplace trait + client + OCI scan layer.
//!
//! Lives in a sibling file so `marketplace.rs` stays under the
//! per-file 500-LOC budget. The mock HTTP server is a hand-rolled
//! `tokio::TcpListener` (same pattern as
//! `argo-marketplace/tests/oci_distribution_roundtrip.rs`) since
//! neither mockito nor wiremock is in the workspace.

use super::oci::{MarketplaceConfig, OciRegistryScan};
use super::{
    MarketplaceClient, MarketplaceError, MarketplaceSkill, MarketplaceSource, SearchOptions,
    StaticIndex, STATIC_INDEX_DATA,
};
use crate::TrustGrade;
use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

struct ErroringSource;

#[async_trait]
impl MarketplaceSource for ErroringSource {
    async fn list_all(&self) -> Result<Vec<MarketplaceSkill>, MarketplaceError> {
        Err(MarketplaceError::Network {
            url: "https://erroring.example/v2/_catalog".into(),
            reason: "simulated failure".into(),
        })
    }
}

struct StubSource(Vec<MarketplaceSkill>);

#[async_trait]
impl MarketplaceSource for StubSource {
    async fn list_all(&self) -> Result<Vec<MarketplaceSkill>, MarketplaceError> {
        Ok(self.0.clone())
    }
}

fn sample(name: &str, tenant: &str) -> MarketplaceSkill {
    MarketplaceSkill {
        name: name.into(),
        tenant: tenant.into(),
        description: format!("desc-{name}"),
        version: "1.0.0".into(),
        trust_grade: TrustGrade::PublishedCommunity,
        last_published_at: "2026-05-10T00:00:00Z".into(),
        oci_ref: format!("ghcr.io/test/{name}:1.0.0"),
    }
}

#[tokio::test]
async fn trait_impl_static_roundtrip() {
    let rows = StaticIndex.list_all().await.expect("static list_all");
    assert_eq!(rows.len(), STATIC_INDEX_DATA.len());
    let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"marketplace-listing-draft"));
    assert!(names.contains(&"listing-photo-alt-text"));
}

#[tokio::test]
async fn client_merges_two_sources_dedupes_by_id() {
    let source_a = StubSource(vec![sample("alpha", "acme"), sample("beta", "acme")]);
    let source_b = StubSource(vec![
        sample("beta", "globex"),
        sample("gamma", "globex"),
    ]);
    let client = MarketplaceClient::new(vec![Box::new(source_a), Box::new(source_b)]);
    let rows = client.list_all().await;
    let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names.len(), 3);
    assert_eq!(names, ["alpha", "beta", "gamma"]);
    let beta = rows.iter().find(|r| r.name == "beta").unwrap();
    assert_eq!(beta.tenant, "acme", "first-source-wins on dedupe");
}

#[tokio::test]
async fn client_falls_back_when_source_errors() {
    let client = MarketplaceClient::new(vec![
        Box::new(ErroringSource),
        Box::new(StubSource(vec![sample("delta", "acme")])),
    ]);
    let rows = client.list_all().await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "delta");
    let searched = client
        .search("delta", &SearchOptions::default())
        .await;
    assert_eq!(searched.len(), 1);
    assert_eq!(searched[0].name, "delta");
}

#[test]
fn toml_parse_empty_file() {
    let cfg = MarketplaceConfig::parse_toml("").expect("empty TOML parses");
    assert!(cfg.registries.is_empty());
}

#[test]
fn toml_parse_multi_registry() {
    let body = r#"
[[registry]]
url = "https://skycot.cr"
namespace = "skills"

[[registry]]
url = "https://ghcr.io"
namespace = "skycot"
"#;
    let cfg = MarketplaceConfig::parse_toml(body).expect("multi-registry TOML parses");
    assert_eq!(cfg.registries.len(), 2);
    assert_eq!(cfg.registries[0].url, "https://skycot.cr");
    assert_eq!(cfg.registries[0].namespace, "skills");
    assert_eq!(cfg.registries[1].namespace, "skycot");
}

/// Integration test: spin up a minimal mock OCI registry and verify
/// `OciRegistryScan::list_all` walks `_catalog` → `tags/list` →
/// manifest annotations into a real [`MarketplaceSkill`]. Documents
/// the HTTP shape the production code expects.
#[tokio::test]
async fn oci_registry_scan_hydrates_from_mock_server() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            tokio::spawn(async move {
                handle_one(&mut stream).await;
            });
        }
    });

    let scan = OciRegistryScan::new(format!("http://127.0.0.1:{port}"), "skills".into());
    let rows = scan.list_all().await.expect("mock catalog list_all");
    server.abort();

    assert_eq!(rows.len(), 1, "namespace filter excludes other/junk");
    let row = &rows[0];
    assert_eq!(row.name, "kpi-tracker");
    assert_eq!(row.description, "Tracks KPIs from a worksheet.");
    assert_eq!(row.tenant, "acme");
    assert_eq!(row.trust_grade, TrustGrade::PublishedVerified);
    assert_eq!(row.oci_ref, "skills/kpi-tracker:v1");
}

async fn handle_one(stream: &mut tokio::net::TcpStream) {
    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf).await.unwrap_or(0);
    let req = String::from_utf8_lossy(&buf[..n]);
    let first_line = req.lines().next().unwrap_or("");
    let path = first_line.split_whitespace().nth(1).unwrap_or("/");

    let body = if path.starts_with("/v2/_catalog") {
        r#"{"repositories":["skills/kpi-tracker","other/ignored"]}"#.to_string()
    } else if path == "/v2/skills/kpi-tracker/tags/list" {
        r#"{"name":"skills/kpi-tracker","tags":["v0","v1"]}"#.to_string()
    } else if path == "/v2/skills/kpi-tracker/manifests/v1" {
        r#"{"annotations":{"org.opencontainers.image.title":"kpi-tracker","org.opencontainers.image.description":"Tracks KPIs from a worksheet.","org.opencontainers.image.version":"1.0.0","org.opencontainers.image.created":"2026-05-10T00:00:00Z","org.skycot.trust_grade":"published_verified","org.skycot.tenant":"acme"}}"#.to_string()
    } else {
        let _ = stream
            .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n")
            .await;
        return;
    };
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(resp.as_bytes()).await;
}
