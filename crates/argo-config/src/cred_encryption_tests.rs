use super::*;
use std::sync::{Mutex, OnceLock};

fn env_mutex() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn restore_env(key: &str, prior: Option<String>) {
    match prior {
        Some(value) => env::set_var(key, value),
        None => env::remove_var(key),
    }
}

fn with_master_key<F: FnOnce(&argo_credentials::MasterKey)>(f: F) {
    let _guard = env_mutex().lock().unwrap_or_else(|e| e.into_inner());
    let key = argo_credentials::MasterKey::generate();
    let prior = env::var(argo_credentials::MASTER_KEY_ENV).ok();
    env::set_var(argo_credentials::MASTER_KEY_ENV, key.as_age_secret());
    f(&key);
    restore_env(argo_credentials::MASTER_KEY_ENV, prior);
}

#[test]
fn load_reads_encrypted_entry() {
    with_master_key(|key| {
        let dir = tempfile::tempdir().unwrap();
        let blob = argo_credentials::encrypt_value("secret", key).unwrap();
        let body = format!("[credentials.A_TOKEN]\nvalue = \"{blob}\"\n");
        fs::create_dir_all(dir.path().join(ARGO_DIR)).unwrap();
        fs::write(credentials_path(dir.path()), body).unwrap();

        let loaded = load(dir.path()).unwrap();
        assert_eq!(loaded.get("A_TOKEN").map(String::as_str), Some("secret"));
    });
}

#[test]
fn load_accepts_mixed_plain_and_encrypted_entries() {
    with_master_key(|key| {
        let dir = tempfile::tempdir().unwrap();
        let blob = argo_credentials::encrypt_value("wrapped", key).unwrap();
        let body = format!(
            "[credentials]\nPLAIN_TOKEN = \"plain\"\n\n[credentials.WRAPPED_TOKEN]\nvalue = \"{blob}\"\n"
        );
        fs::create_dir_all(dir.path().join(ARGO_DIR)).unwrap();
        fs::write(credentials_path(dir.path()), body).unwrap();

        let loaded = load(dir.path()).unwrap();
        assert_eq!(loaded.get("PLAIN_TOKEN").map(String::as_str), Some("plain"));
        assert_eq!(
            loaded.get("WRAPPED_TOKEN").map(String::as_str),
            Some("wrapped")
        );
    });
}

#[test]
fn get_from_workspace_decrypts_encrypted_file_entry() {
    with_master_key(|key| {
        let dir = tempfile::tempdir().unwrap();
        let name = "ARGO_TEST_ENCRYPTED_GET_TOKEN";
        env::remove_var(name);
        let blob = argo_credentials::encrypt_value("from-file", key).unwrap();
        let body = format!("[credentials.{name}]\nvalue = \"{blob}\"\n");
        fs::create_dir_all(dir.path().join(ARGO_DIR)).unwrap();
        fs::write(credentials_path(dir.path()), body).unwrap();

        assert_eq!(
            get_unscoped_from_workspace(dir.path(), name).unwrap(),
            "from-file"
        );
    });
}

#[test]
fn encrypted_entry_without_master_key_returns_typed_error() {
    let _guard = env_mutex().lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let prior_key = env::var(argo_credentials::MASTER_KEY_ENV).ok();
    let prior_home = env::var("HOME").ok();
    env::remove_var(argo_credentials::MASTER_KEY_ENV);
    env::set_var("HOME", home.path());
    fs::create_dir_all(dir.path().join(ARGO_DIR)).unwrap();
    fs::write(
        credentials_path(dir.path()),
        "[credentials.A_TOKEN]\nvalue = \"not-used\"\n",
    )
    .unwrap();

    let err = load(dir.path()).unwrap_err();
    assert!(format!("{err}").contains("master key not configured"));

    restore_env(argo_credentials::MASTER_KEY_ENV, prior_key);
    restore_env("HOME", prior_home);
}

#[test]
fn malformed_encrypted_blob_returns_typed_error() {
    with_master_key(|_| {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(ARGO_DIR)).unwrap();
        fs::write(
            credentials_path(dir.path()),
            "[credentials.A_TOKEN]\nvalue = \"not-base64%%%\"\n",
        )
        .unwrap();

        let err = load(dir.path()).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("decrypt A_TOKEN"), "msg: {msg}");
        assert!(msg.contains("base64 decode"), "msg: {msg}");
    });
}

#[test]
fn set_tenant_encrypts_byok_key_at_rest() {
    // Acceptance (pricing-byok-encrypt-keys-at-rest): a tenant-scoped key
    // written via `set_tenant` lands ENCRYPTED — the plaintext secret must
    // not appear on disk — and the read path decrypts it back unchanged.
    with_master_key(|_| {
        let dir = tempfile::tempdir().unwrap();
        let tenant = argo_types::TenantId::new();
        let secret = "sk-ant-byok-supersecret-do-not-leak";
        env::remove_var("ANTHROPIC_API_KEY");

        set_tenant(dir.path(), &tenant, "ANTHROPIC_API_KEY", secret).unwrap();

        // On-disk TOML must NOT contain the plaintext secret.
        let body = fs::read_to_string(credentials_path(dir.path())).unwrap();
        assert!(
            !body.contains(secret),
            "plaintext provider key leaked to disk: {body}"
        );

        // Read path (used by `resolve_provider_api_key` via `load_tenant`)
        // transparently decrypts back to the original.
        let via_get = get_from_workspace(dir.path(), &tenant, "ANTHROPIC_API_KEY").unwrap();
        assert_eq!(via_get, secret);
        let loaded = load_tenant(dir.path(), &tenant).unwrap();
        assert_eq!(
            loaded.get("ANTHROPIC_API_KEY").map(String::as_str),
            Some(secret)
        );
    });
}

#[test]
fn set_tenant_falls_back_to_plain_without_master_key() {
    // Key-less workspace: no master key configured → graceful plaintext
    // fallback (with a one-shot warn) so dev/test environments keep
    // round-tripping. The read path handles the plain shape.
    let _guard = env_mutex().lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let tenant = argo_types::TenantId::new();
    let prior_root = env::var(argo_credentials::WORKSPACE_ROOT_KEY_ENV).ok();
    let prior_key = env::var(argo_credentials::MASTER_KEY_ENV).ok();
    let prior_home = env::var("HOME").ok();
    env::remove_var(argo_credentials::WORKSPACE_ROOT_KEY_ENV);
    env::remove_var(argo_credentials::MASTER_KEY_ENV);
    env::set_var("HOME", home.path());
    env::remove_var("PLAIN_FALLBACK_TOKEN");

    set_tenant(dir.path(), &tenant, "PLAIN_FALLBACK_TOKEN", "plain-val").unwrap();

    let body = fs::read_to_string(credentials_path(dir.path())).unwrap();
    assert!(
        body.contains("plain-val"),
        "expected plaintext fallback when no master key configured: {body}"
    );
    let got = get_from_workspace(dir.path(), &tenant, "PLAIN_FALLBACK_TOKEN").unwrap();
    assert_eq!(got, "plain-val");

    restore_env(argo_credentials::WORKSPACE_ROOT_KEY_ENV, prior_root);
    restore_env(argo_credentials::MASTER_KEY_ENV, prior_key);
    restore_env("HOME", prior_home);
}

#[test]
fn migrate_plaintext_format_round_trips_through_load() {
    with_master_key(|key| {
        let dir = tempfile::tempdir().unwrap();
        set(dir.path(), "ANTHROPIC_API_KEY", "sk-test").unwrap();

        argo_credentials::migrate_plaintext_in_place(&credentials_path(dir.path()), key).unwrap();

        let body = fs::read_to_string(credentials_path(dir.path())).unwrap();
        assert!(!body.contains("sk-test"), "plaintext leaked: {body}");
        let loaded = load(dir.path()).unwrap();
        assert_eq!(
            loaded.get("ANTHROPIC_API_KEY").map(String::as_str),
            Some("sk-test")
        );
    });
}
