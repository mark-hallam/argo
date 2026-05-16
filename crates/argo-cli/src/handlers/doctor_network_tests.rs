use super::*;
use tempfile::tempdir;

struct FakeProbe;

#[async_trait::async_trait]
impl NetworkProbe for FakeProbe {
    async fn dns(&self, host: &str) -> Value {
        json!({"host": host, "status": "OK", "detail": "127.0.0.1"})
    }

    async fn tls(&self, host: &str) -> Value {
        json!({"host": host, "status": "OK", "detail": "expires 2099-01-01"})
    }

    async fn auth(&self, _cwd: &Path, target: &AuthTarget) -> Value {
        if target.provider == "openai" {
            json!({"provider": target.provider, "host": target.host, "status": "SKIP", "detail": "no key configured"})
        } else {
            json!({"provider": target.provider, "host": target.host, "status": "OK", "status_code": 200, "detail": "HTTP 200"})
        }
    }
}

#[tokio::test]
async fn skip_network_returns_empty_groups() {
    let dir = tempdir().unwrap();
    let value = collect_with_probe(dir.path(), true, &FakeProbe).await;
    assert_eq!(value["skipped"], true);
    assert!(value["dns"].as_array().unwrap().is_empty());
    assert!(value["tls"].as_array().unwrap().is_empty());
    assert!(value["auth"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn default_collect_includes_dns_tls_and_auth_groups() {
    let dir = tempdir().unwrap();
    let value = collect_with_probe(dir.path(), false, &FakeProbe).await;
    assert_eq!(value["skipped"], false);
    assert_eq!(value["dns"].as_array().unwrap().len(), 3);
    assert_eq!(value["tls"].as_array().unwrap().len(), 3);
    assert_eq!(value["auth"].as_array().unwrap().len(), 2);
    assert_eq!(value["auth"][1]["status"], "SKIP");
}

#[tokio::test]
async fn configured_additional_hosts_are_included_once() {
    let dir = tempdir().unwrap();
    std::fs::write(
        dir.path().join("argo.toml"),
        "[doctor]\nadditional_hosts = [\"api.example.com\", \"skycot.com\"]\n",
    )
    .unwrap();
    let value = collect_with_probe(dir.path(), false, &FakeProbe).await;
    let hosts: Vec<_> = value["dns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["host"].as_str().unwrap())
        .collect();
    assert!(hosts.contains(&"api.example.com"));
    assert_eq!(hosts.iter().filter(|h| **h == "skycot.com").count(), 1);
}

#[test]
fn valid_host_rejects_shell_and_label_edge_cases() {
    assert!(valid_host("api.example.com"));
    assert!(!valid_host("api.example.com;touch"));
    assert!(!valid_host("-api.example.com"));
    assert!(!valid_host("api..example.com"));
}
