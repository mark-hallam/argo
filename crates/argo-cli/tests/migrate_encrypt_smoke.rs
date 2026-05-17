//! E2E smoke for `argo migrate encrypt`
//! (`mission-sqlcipher-3-migrate-cli`).
//!
//! Three test cases run unconditionally:
//! 1. `--dry-run --json` reports the planned actions without touching
//!    the source file or creating a backup.
//! 2. Missing `SKYCOT_DB_KEY_BASE64` fails preflight with a clear
//!    error.
//! 3. A non-existent `--db` fails preflight before any other work.
//!
//! A fourth test that exercises the full encrypt-then-verify pipeline
//! requires the binary to be linked against SQLCipher
//! (`mission-sqlcipher-1-libsqlite3-patch`). It detects the linked
//! driver at start and short-circuits with a `println!` when run
//! against a stock-sqlite build, so it stays green on both
//! pre/post-PR1 main.

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde_json::Value;
use std::path::PathBuf;
use std::process::{Command, Output};

fn argo_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo")
}

fn run(args: &[&str], home: &std::path::Path, key_env: Option<&str>) -> Output {
    let mut cmd = Command::new(argo_bin());
    cmd.env("HOME", home).env_remove("SKYCOT_DB_KEY_BASE64");
    if let Some(k) = key_env {
        cmd.env("SKYCOT_DB_KEY_BASE64", k);
    }
    cmd.args(args).output().expect("spawn argo")
}

fn seed_unencrypted_db(path: &std::path::Path) {
    let url = format!("sqlite://{}?mode=rwc", path.display());
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async move {
        use sqlx::{Connection, SqliteConnection};
        let mut conn = SqliteConnection::connect(&url).await.unwrap();
        sqlx::query("CREATE TABLE widgets (id INTEGER PRIMARY KEY, label TEXT)")
            .execute(&mut conn)
            .await
            .unwrap();
        for i in 0..3 {
            sqlx::query("INSERT INTO widgets (label) VALUES (?1)")
                .bind(format!("widget-{i}"))
                .execute(&mut conn)
                .await
                .unwrap();
        }
        conn.close().await.unwrap();
    });
}

fn b64_key() -> String {
    B64.encode([0x11u8; 32])
}

#[test]
fn dry_run_emits_plan_without_writing() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let db_path = work.path().join("session.db");
    seed_unencrypted_db(&db_path);
    let before_meta = std::fs::metadata(&db_path).unwrap();

    let out = run(
        &[
            "migrate",
            "encrypt",
            "--db",
            db_path.to_str().unwrap(),
            "--dry-run",
            "--json",
        ],
        home.path(),
        Some(&b64_key()),
    );
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let v: Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(v["ok"], true);
    assert_eq!(v["migrate_encrypt"]["dry_run"], true);
    assert_eq!(v["migrate_encrypt"]["already_encrypted"], false);

    // Source file is untouched; no backup directory created.
    let after_meta = std::fs::metadata(&db_path).unwrap();
    assert_eq!(before_meta.len(), after_meta.len());
    let backup_root: PathBuf = home.path().join(".argo").join("backups");
    assert!(!backup_root.exists(), "dry-run must not create backups");
}

#[test]
fn missing_key_env_fails_with_clear_error() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let db_path = work.path().join("session.db");
    seed_unencrypted_db(&db_path);

    let out = run(
        &["migrate", "encrypt", "--db", db_path.to_str().unwrap()],
        home.path(),
        None,
    );
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("SKYCOT_DB_KEY_BASE64"),
        "expected env-var error in stderr, got: {stderr}"
    );
}

#[test]
fn missing_db_fails_preflight() {
    let home = tempfile::tempdir().unwrap();
    let ghost = home.path().join("does-not-exist.db");
    let out = run(
        &[
            "migrate",
            "encrypt",
            "--db",
            ghost.to_str().unwrap(),
            "--dry-run",
        ],
        home.path(),
        Some(&b64_key()),
    );
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.to_lowercase().contains("does not exist"),
        "expected missing-source error, got: {stderr}"
    );
}

#[test]
fn full_encrypt_when_sqlcipher_linked() {
    // Skip on builds that aren't SQLCipher-linked (PR 1 not yet
    // merged or feature unset). The dry-run + preflight tests above
    // exercise the surface that works everywhere.
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let db_path = work.path().join("session.db");
    seed_unencrypted_db(&db_path);

    if !sqlcipher_linked() {
        println!(
            "skipping full-encrypt smoke: cipher_version() unavailable \
             (mission-sqlcipher-1-libsqlite3-patch not yet merged)"
        );
        return;
    }

    let out = run(
        &[
            "migrate",
            "encrypt",
            "--db",
            db_path.to_str().unwrap(),
            "--json",
            "--yes",
        ],
        home.path(),
        Some(&b64_key()),
    );
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(v["ok"], true);
    let tables = v["migrate_encrypt"]["tables"].as_array().expect("tables");
    let widgets = tables
        .iter()
        .find(|t| t["name"] == "widgets")
        .expect("widgets table verified");
    assert_eq!(widgets["source_rows"], 3);
    assert_eq!(widgets["encrypted_rows"], 3);
}

fn sqlcipher_linked() -> bool {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        use sqlx::{Connection, SqliteConnection};
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let url = format!("sqlite://{}?mode=rwc", tmp.path().display());
        let Ok(mut conn) = SqliteConnection::connect(&url).await else {
            return false;
        };
        let v: Result<(String,), _> = sqlx::query_as("SELECT cipher_version()")
            .fetch_one(&mut conn)
            .await;
        let _ = conn.close().await;
        matches!(v, Ok((s,)) if !s.trim().is_empty())
    })
}
