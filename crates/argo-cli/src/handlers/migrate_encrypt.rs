//! `argo migrate encrypt` — convert an unencrypted SQLite database
//! to a SQLCipher-encrypted one with a guaranteed backup, row-count
//! verification, and an atomic swap
//! (`mission-sqlcipher-3-migrate-cli`).
//!
//! Flow: preflight → backup → `ATTACH ... KEY` + `sqlcipher_export`
//! → reopen-with-key + row-count compare → atomic swap. Idempotent.

use crate::cli::cli_subcommands_migrate::{MigrateEncryptArgs, MigrateKeySource};
use argo_types::{ArgoError, ArgoResult};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use chrono::Utc;
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Connection, Row, SqliteConnection};
use std::path::{Path, PathBuf};
use std::str::FromStr;

const KEY_ENV_VAR: &str = "SKYCOT_DB_KEY_BASE64";

#[derive(Debug, Clone, Serialize)]
pub struct MigrateEncryptOut {
    pub source: String,
    pub backup: Option<String>,
    pub encrypted: String,
    pub tables: Vec<TableRowCheck>,
    pub already_encrypted: bool,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TableRowCheck {
    pub name: String,
    pub source_rows: i64,
    pub encrypted_rows: i64,
}

pub async fn run(args: MigrateEncryptArgs) -> ArgoResult<()> {
    let json = args.json;
    let out = migrate_encrypt(args).await?;
    if json {
        let envelope = serde_json::json!({"ok": true, "migrate_encrypt": out});
        println!(
            "{}",
            serde_json::to_string(&envelope)
                .map_err(|e| ArgoError::Provider(format!("migrate-encrypt JSON: {e}")))?
        );
    } else if out.dry_run {
        println!("dry-run: would encrypt {}", out.source);
    } else if out.already_encrypted {
        println!("{} is already encrypted; no-op", out.source);
    } else {
        let backup = out.backup.as_deref().unwrap_or("<none>");
        println!(
            "migration complete: {} ({} tables verified). backup at {}. \
             original kept at {}.pre-encrypt.bak — remove after verification.",
            out.source,
            out.tables.len(),
            backup,
            out.source,
        );
    }
    Ok(())
}

async fn migrate_encrypt(args: MigrateEncryptArgs) -> ArgoResult<MigrateEncryptOut> {
    let source = args.db.clone();
    if !source.exists() {
        return Err(ArgoError::Io(format!(
            "migrate-encrypt: source database does not exist: {}",
            source.display()
        )));
    }
    let source_str = source.to_string_lossy().to_string();

    if is_encrypted(&source).await? {
        return Ok(MigrateEncryptOut {
            source: source_str.clone(),
            backup: None,
            encrypted: source_str,
            tables: Vec::new(),
            already_encrypted: true,
            dry_run: args.dry_run,
        });
    }

    let source_bytes = std::fs::metadata(&source)?.len();
    check_free_space(&source, source_bytes)?;
    let hex_key = resolve_hex_key(args.key_source)?;

    if args.dry_run {
        return Ok(MigrateEncryptOut {
            source: source_str.clone(),
            backup: Some(plan_backup_path(&source)?.to_string_lossy().into_owned()),
            encrypted: source_str,
            tables: Vec::new(),
            already_encrypted: false,
            dry_run: true,
        });
    }

    let backup_path = make_backup(&source)?;
    let tmp_dest = source.with_extension("encrypted.tmp");
    if tmp_dest.exists() {
        std::fs::remove_file(&tmp_dest)?;
    }
    let tables = match encrypt_and_verify(&source, &tmp_dest, &hex_key).await {
        Ok(t) => t,
        Err(e) => {
            let _ = std::fs::remove_file(&tmp_dest);
            return Err(e);
        }
    };
    let pre_backup = source.with_extension(format!(
        "{}.pre-encrypt.bak",
        source.extension().and_then(|s| s.to_str()).unwrap_or("db"),
    ));
    std::fs::rename(&source, &pre_backup)?;
    std::fs::rename(&tmp_dest, &source)?;

    Ok(MigrateEncryptOut {
        source: source_str.clone(),
        backup: Some(backup_path.to_string_lossy().into_owned()),
        encrypted: source_str,
        tables,
        already_encrypted: false,
        dry_run: false,
    })
}

async fn is_encrypted(path: &Path) -> ArgoResult<bool> {
    // Plain-text SQLite open. If it fails with a "file is not a
    // database" / "encrypted" signature, treat as already encrypted.
    let opts = SqliteConnectOptions::from_str(&path.to_string_lossy())
        .map_err(|e| ArgoError::Provider(format!("migrate-encrypt: open opts: {e}")))?
        .create_if_missing(false);
    match SqliteConnection::connect_with(&opts).await {
        Ok(mut conn) => {
            let v: Result<(String,), _> = sqlx::query_as("SELECT cipher_version()")
                .fetch_one(&mut conn)
                .await;
            let _ = conn.close().await;
            Ok(matches!(v, Ok((s,)) if !s.trim().is_empty()))
        }
        Err(e) => {
            let s = e.to_string().to_ascii_lowercase();
            if s.contains("not a database") || s.contains("encrypted") {
                Ok(true)
            } else {
                Err(ArgoError::Provider(format!(
                    "migrate-encrypt: cannot probe source DB: {e}"
                )))
            }
        }
    }
}

fn check_free_space(source: &Path, source_bytes: u64) -> ArgoResult<()> {
    let parent = source.parent().unwrap_or(Path::new("."));
    let free = free_space(parent).unwrap_or(u64::MAX);
    if free < source_bytes.saturating_mul(2) {
        return Err(ArgoError::Io(format!(
            "migrate-encrypt: need ≥{} bytes free in {}; have {}",
            source_bytes.saturating_mul(2),
            parent.display(),
            free,
        )));
    }
    Ok(())
}

#[cfg(unix)]
fn free_space(p: &Path) -> Option<u64> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let c = CString::new(p.as_os_str().as_bytes()).ok()?;
    // SAFETY: POSIX statvfs; pointer from stack-local zeroed struct.
    unsafe {
        let mut stat: libc::statvfs = std::mem::zeroed();
        if libc::statvfs(c.as_ptr(), &mut stat) != 0 {
            return None;
        }
        Some(stat.f_bavail as u64 * stat.f_frsize as u64)
    }
}

#[cfg(not(unix))]
fn free_space(_p: &Path) -> Option<u64> {
    None
}

fn resolve_hex_key(_source: MigrateKeySource) -> ArgoResult<String> {
    // Both `env` and `keyring` flow through the env var today; PR 4
    // extends `Keyring` to call the per-tenant `argo_config::cred::
    // db_key::get_or_create_db_key` API landing in PR 2.
    let raw = std::env::var(KEY_ENV_VAR).map_err(|_| {
        ArgoError::Schema(format!(
            "migrate-encrypt: ${KEY_ENV_VAR} must be set to a base64-encoded 32-byte key"
        ))
    })?;
    let bytes = B64
        .decode(raw.trim())
        .map_err(|e| ArgoError::Schema(format!("migrate-encrypt: invalid base64 key: {e}")))?;
    if bytes.len() != 32 {
        return Err(ArgoError::Schema(format!(
            "migrate-encrypt: key must decode to 32 bytes (got {})",
            bytes.len()
        )));
    }
    Ok(hex_encode(&bytes))
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn plan_backup_path(source: &Path) -> ArgoResult<PathBuf> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| ArgoError::Io("migrate-encrypt: $HOME unset".into()))?;
    let stamp = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let file_name = source
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "session.db".to_string());
    Ok(home
        .join(".argo")
        .join("backups")
        .join(stamp)
        .join(format!("{file_name}.unencrypted.db")))
}

fn make_backup(source: &Path) -> ArgoResult<PathBuf> {
    let dest = plan_backup_path(source)?;
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(source, &dest)?;
    let src_hash = sha256_file(source)?;
    let dst_hash = sha256_file(&dest)?;
    if src_hash != dst_hash {
        let _ = std::fs::remove_file(&dest);
        return Err(ArgoError::Io(format!(
            "migrate-encrypt: backup checksum mismatch ({src_hash} vs {dst_hash})"
        )));
    }
    Ok(dest)
}

fn sha256_file(path: &Path) -> ArgoResult<String> {
    use std::io::Read;
    let mut f = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex_encode(&hasher.finalize()))
}

async fn user_table_names(conn: &mut SqliteConnection) -> ArgoResult<Vec<String>> {
    let rows = sqlx::query(
        "SELECT name FROM sqlite_master \
         WHERE type='table' AND name NOT LIKE 'sqlite_%' \
         ORDER BY name",
    )
    .fetch_all(&mut *conn)
    .await
    .map_err(|e| ArgoError::Provider(format!("migrate-encrypt: list tables: {e}")))?;
    Ok(rows.into_iter().map(|r| r.get::<String, _>(0)).collect())
}

async fn count_rows(conn: &mut SqliteConnection, table: &str) -> ArgoResult<i64> {
    // Table names come from sqlite_master, never user input. Quote
    // to defuse reserved-word collisions.
    let stmt = format!("SELECT COUNT(*) FROM \"{}\"", table.replace('"', "\"\""));
    let (n,): (i64,) = sqlx::query_as(&stmt)
        .fetch_one(&mut *conn)
        .await
        .map_err(|e| ArgoError::Provider(format!("migrate-encrypt: count {table}: {e}")))?;
    Ok(n)
}

async fn encrypt_and_verify(
    source: &Path,
    tmp_dest: &Path,
    hex_key: &str,
) -> ArgoResult<Vec<TableRowCheck>> {
    let opts = SqliteConnectOptions::from_str(&source.to_string_lossy())
        .map_err(|e| ArgoError::Provider(format!("migrate-encrypt: open opts: {e}")))?
        .create_if_missing(false);
    let mut conn = SqliteConnection::connect_with(&opts)
        .await
        .map_err(|e| ArgoError::Provider(format!("migrate-encrypt: open source: {e}")))?;
    let source_names = user_table_names(&mut conn).await?;
    let mut source_counts: Vec<(String, i64)> = Vec::with_capacity(source_names.len());
    for n in &source_names {
        source_counts.push((n.clone(), count_rows(&mut conn, n).await?));
    }

    let dest_lit = tmp_dest.to_string_lossy().replace('\'', "''");
    // audit-allow: validated SQLCipher PRAGMA literal (hex-only key,
    // single-quote-escaped path).
    let attach = format!("ATTACH DATABASE '{dest_lit}' AS encrypted KEY \"x'{hex_key}'\"");
    sqlx::query(&attach)
        .execute(&mut conn)
        .await
        .map_err(|e| ArgoError::Provider(format!("migrate-encrypt: ATTACH: {e}")))?;
    sqlx::query("SELECT sqlcipher_export('encrypted')")
        .fetch_optional(&mut conn)
        .await
        .map_err(|e| ArgoError::Provider(format!("migrate-encrypt: sqlcipher_export: {e}")))?;
    sqlx::query("DETACH DATABASE encrypted")
        .execute(&mut conn)
        .await
        .map_err(|e| ArgoError::Provider(format!("migrate-encrypt: DETACH: {e}")))?;
    let _ = conn.close().await;

    let dest_opts = SqliteConnectOptions::from_str(&tmp_dest.to_string_lossy())
        .map_err(|e| ArgoError::Provider(format!("migrate-encrypt: dest open opts: {e}")))?
        .create_if_missing(false);
    let mut dest_conn = SqliteConnection::connect_with(&dest_opts)
        .await
        .map_err(|e| ArgoError::Provider(format!("migrate-encrypt: open dest: {e}")))?;
    sqlx::query(&format!("PRAGMA key = \"x'{hex_key}'\""))
        .execute(&mut dest_conn)
        .await
        .map_err(|e| ArgoError::Provider(format!("migrate-encrypt: dest PRAGMA key: {e}")))?;
    let cv: (String,) = sqlx::query_as("SELECT cipher_version()")
        .fetch_one(&mut dest_conn)
        .await
        .map_err(|e| {
            ArgoError::Provider(format!(
                "migrate-encrypt: cipher_version() unavailable — \
                 binary not linked against SQLCipher (PR 1 prereq): {e}"
            ))
        })?;
    if cv.0.trim().is_empty() {
        return Err(ArgoError::Provider(
            "migrate-encrypt: cipher_version() returned empty — encryption did not take".into(),
        ));
    }
    let mut tables = Vec::with_capacity(source_counts.len());
    for (name, src_n) in source_counts {
        let dst_n = count_rows(&mut dest_conn, &name).await?;
        if dst_n != src_n {
            return Err(ArgoError::Provider(format!(
                "migrate-encrypt: row-count mismatch for {name}: source={src_n}, encrypted={dst_n}"
            )));
        }
        tables.push(TableRowCheck {
            name,
            source_rows: src_n,
            encrypted_rows: dst_n,
        });
    }
    let _ = dest_conn.close().await;
    Ok(tables)
}
