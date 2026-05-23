//! Cross-tenant negative + legacy-migration tests for the gcal OAuth
//! token cache. Kept as a separate file from `client.rs` to keep the
//! parent under the 500-LOC ceiling; declared as a `#[path = ...]`
//! child module so it sees `client.rs`'s private items.
use super::*;
use tempfile::TempDir;

fn refresh_with(token: &str) -> RefreshResponse {
    RefreshResponse {
        access_token: token.into(),
        expires_in: Some(3600),
    }
}

#[test]
fn cache_path_includes_tenant_segment() {
    let p = token_cache_path(Path::new("/work"), "tenant-a");
    assert!(p.ends_with(".argo/connectors/tenant-a/gcal/access_token.json"));
}

#[test]
fn legacy_cache_path_matches_pre_tenant_shape() {
    let p = legacy_cache_path(Path::new("/work"));
    assert!(p.ends_with(".argo/connectors/gcal/access_token.json"));
}

// Cross-tenant negative: tenant_b must NEVER read tenant_a's cached
// token. Pre-fix this returned `tok_a` for both reads because both
// tenants shared `<cwd>/.argo/connectors/gcal/access_token.json`.
#[test]
fn cross_tenant_token_isolation() {
    let tmp = TempDir::new().expect("tempdir");
    let cwd = tmp.path();
    write_cached_access_token(cwd, "tenant-a", &refresh_with("tok_a")).unwrap();
    write_cached_access_token(cwd, "tenant-b", &refresh_with("tok_b")).unwrap();

    assert_eq!(
        read_cached_access_token(cwd, "tenant-a").unwrap(),
        Some("tok_a".to_string())
    );
    assert_eq!(
        read_cached_access_token(cwd, "tenant-b").unwrap(),
        Some("tok_b".to_string())
    );

    let path_a = token_cache_path(cwd, "tenant-a");
    let path_b = token_cache_path(cwd, "tenant-b");
    assert_ne!(path_a, path_b);
    assert!(path_a.exists());
    assert!(path_b.exists());
}

// Migration on first read: legacy cwd-shared file moves to legacy/
// and read returns None so the caller refreshes a fresh tenant-keyed
// token rather than serving the shared one.
#[test]
fn legacy_cache_is_quarantined_on_first_read() {
    let tmp = TempDir::new().expect("tempdir");
    let cwd = tmp.path();
    let legacy = legacy_cache_path(cwd);
    std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();
    let leaked = AccessTokenCache {
        access_token: "leaked".into(),
        expires_at: Utc::now().timestamp() + 3600,
    };
    std::fs::write(&legacy, serde_json::to_string(&leaked).unwrap()).unwrap();
    assert!(legacy.exists());

    let result = read_cached_access_token(cwd, "tenant-a").unwrap();
    assert!(
        result.is_none(),
        "legacy token must NOT be served to tenant-a",
    );
    assert!(!legacy.exists(), "legacy file should be moved");
    assert!(
        legacy_quarantine_path(cwd).exists(),
        "legacy file should land under legacy/",
    );
}
