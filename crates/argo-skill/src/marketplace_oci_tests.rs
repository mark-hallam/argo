use super::{fetch_all_pages, MarketplaceConfig, OciRegistryScan, DEFAULT_MAX_CATALOG_ITEMS};
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
"#,
    )
    .expect("marketplace TOML parses");
    assert_eq!(cfg.max_catalog_items, 250);

    let defaulted = MarketplaceConfig::parse_toml("").expect("empty TOML parses");
    assert_eq!(defaulted.max_catalog_items, DEFAULT_MAX_CATALOG_ITEMS);
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
