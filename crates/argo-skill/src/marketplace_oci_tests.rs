use super::{
    fetch_all_pages, MarketplaceConfig, MarketplaceTagPolicy, OciRegistryScan,
    DEFAULT_MAX_CATALOG_ITEMS,
};
use crate::marketplace::MarketplaceSource;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[test]
fn toml_parse_catalog_cap() {
    let cfg = MarketplaceConfig::parse_toml(
        r#"
max_catalog_items = 250

[[registry]]
url = "https://skycot.cr"
namespace = "skills"
tag_policy = "alphabetical_last"
"#,
    )
    .expect("marketplace TOML parses");
    assert_eq!(cfg.max_catalog_items, 250);
    assert_eq!(
        cfg.registries[0].tag_policy,
        MarketplaceTagPolicy::AlphabeticalLast
    );

    let defaulted = MarketplaceConfig::parse_toml("").expect("empty TOML parses");
    assert_eq!(defaulted.max_catalog_items, DEFAULT_MAX_CATALOG_ITEMS);
}

/// Loading a malformed `marketplace.toml` from disk must surface the
/// on-disk path so the operator knows which file is broken.
#[test]
fn load_or_default_includes_file_path_on_parse_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("marketplace.toml");
    std::fs::write(&path, "this is = not [valid").expect("write");
    let err = MarketplaceConfig::load_or_default(&path).unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains(&path.display().to_string()),
        "expected error to include {} — got {msg}",
        path.display()
    );
}

#[tokio::test]
async fn list_all_follows_catalog_link_pages() {
    let (base, server) = spawn_registry(3).await;
    let scan = OciRegistryScan::new(base, "skills".into());
    let rows = scan.list_all().await.expect("paginated list_all");
    server.abort();

    assert_eq!(rows.len(), 300);
    assert_eq!(rows[0].name, "skill-000");
    assert_eq!(rows[299].name, "skill-299");
}

#[tokio::test]
async fn fetch_all_pages_stops_at_configured_cap() {
    let (base, server) = spawn_registry(5).await;
    let client = reqwest::Client::new();
    let rows = fetch_all_pages(&client, &format!("{base}/v2/_catalog?n=100"), 200)
        .await
        .expect("capped pagination");
    server.abort();

    assert_eq!(rows.len(), 200);
    assert_eq!(rows[0], "skills/skill-000");
    assert_eq!(rows[199], "skills/skill-199");
}

#[tokio::test]
async fn timestamp_newest_policy_selects_newest_manifest_created_at() {
    let row = scan_one_with_policy(MarketplaceTagPolicy::TimestampNewest).await;
    assert_eq!(row.version, "2.0.0");
    assert_eq!(row.oci_ref, "skills/tagged:latest");
}

#[tokio::test]
async fn alphabetical_last_policy_preserves_legacy_tag_ordering() {
    let row = scan_one_with_policy(MarketplaceTagPolicy::AlphabeticalLast).await;
    assert_eq!(row.version, "1.0.0");
    assert_eq!(row.oci_ref, "skills/tagged:v1");
}

#[tokio::test]
async fn explicit_policy_selects_named_tag() {
    let row = scan_one_with_policy(MarketplaceTagPolicy::Explicit("beta".into())).await;
    assert_eq!(row.version, "1.5.0");
    assert_eq!(row.oci_ref, "skills/tagged:beta");
}

async fn scan_one_with_policy(
    policy: MarketplaceTagPolicy,
) -> crate::marketplace::MarketplaceSkill {
    let (base, server) = spawn_tag_registry().await;
    let rows =
        OciRegistryScan::with_tag_policy(base, "skills".into(), DEFAULT_MAX_CATALOG_ITEMS, policy)
            .list_all()
            .await
            .expect("tag policy scan");
    server.abort();
    assert_eq!(rows.len(), 1);
    rows.into_iter().next().expect("one row")
}

async fn spawn_registry(page_count: usize) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            tokio::spawn(async move {
                handle_one(&mut stream, page_count).await;
            });
        }
    });
    (format!("http://127.0.0.1:{port}"), server)
}

async fn spawn_tag_registry() -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            tokio::spawn(async move {
                handle_tag_policy_one(&mut stream).await;
            });
        }
    });
    (format!("http://127.0.0.1:{port}"), server)
}

async fn handle_tag_policy_one(stream: &mut tokio::net::TcpStream) {
    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf).await.unwrap_or(0);
    let req = String::from_utf8_lossy(&buf[..n]);
    let path = req
        .lines()
        .next()
        .unwrap_or("")
        .split_whitespace()
        .nth(1)
        .unwrap_or("/");

    let body = if path.starts_with("/v2/_catalog") {
        r#"{"repositories":["skills/tagged"]}"#.to_string()
    } else if path == "/v2/skills/tagged/tags/list" {
        r#"{"tags":["v1","latest","beta"]}"#.to_string()
    } else if path == "/v2/skills/tagged/manifests/v1" {
        tag_manifest("1.0.0", "2026-05-10T00:00:00Z")
    } else if path == "/v2/skills/tagged/manifests/latest" {
        tag_manifest("2.0.0", "2026-05-12T00:00:00Z")
    } else if path == "/v2/skills/tagged/manifests/beta" {
        tag_manifest("1.5.0", "2026-05-11T00:00:00Z")
    } else {
        let _ = stream
            .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n")
            .await;
        return;
    };
    write_ok(stream, "", &body).await;
}

fn tag_manifest(version: &str, created: &str) -> String {
    format!(
        r#"{{"annotations":{{"org.opencontainers.image.title":"tagged","org.opencontainers.image.description":"tag policy test","org.opencontainers.image.version":"{version}","org.opencontainers.image.created":"{created}","org.skycot.tenant":"acme"}}}}"#
    )
}

async fn handle_one(stream: &mut tokio::net::TcpStream, page_count: usize) {
    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf).await.unwrap_or(0);
    let req = String::from_utf8_lossy(&buf[..n]);
    let path = req
        .lines()
        .next()
        .unwrap_or("")
        .split_whitespace()
        .nth(1)
        .unwrap_or("/");

    if path.starts_with("/v2/_catalog") {
        let page = path
            .split("last=page")
            .nth(1)
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        let body = catalog_body(page);
        let link = if page + 1 < page_count {
            format!(
                "Link: </v2/_catalog?n=100&last=page{}>; rel=\"next\"\r\n",
                page + 1
            )
        } else {
            String::new()
        };
        write_ok(stream, &link, &body).await;
    } else if path.ends_with("/tags/list") {
        write_ok(stream, "", r#"{"tags":["v1"]}"#).await;
    } else if path.contains("/manifests/v1") {
        let name = path
            .trim_start_matches("/v2/skills/")
            .trim_end_matches("/manifests/v1");
        let body = format!(
            r#"{{"annotations":{{"org.opencontainers.image.title":"{name}","org.opencontainers.image.description":"paginated skill","org.opencontainers.image.version":"1.0.0","org.opencontainers.image.created":"2026-05-15T00:00:00Z","org.skycot.tenant":"acme"}}}}"#
        );
        write_ok(stream, "", &body).await;
    } else {
        let _ = stream
            .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n")
            .await;
    }
}

fn catalog_body(page: usize) -> String {
    let start = page * 100;
    let repos = (start..start + 100)
        .map(|i| format!(r#""skills/skill-{i:03}""#))
        .collect::<Vec<_>>()
        .join(",");
    format!(r#"{{"repositories":[{repos}]}}"#)
}

async fn write_ok(stream: &mut tokio::net::TcpStream, extra_headers: &str, body: &str) {
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n{extra_headers}Content-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(resp.as_bytes()).await;
}
