//! Periodic evidence-archival for audit-chain rows (#228, Tier S).
//!
//! Archives rows older than `ArchiveConfig::retention_days` into a
//! `.tar.gz` bundle indexed by an append-only `manifest.jsonl`.
//! Optional pruning deletes archived rows from `audit_record` after
//! the bundle is durable. Bundles are operator-readable: `tar -tzf`
//! lists `audit.ndjson` + `chain_metadata.json`, and `jq` works on
//! `audit.ndjson` without `argo` installed.
//!
//! ```no_run
//! # async fn doc() -> argo_types::ArgoResult<()> {
//! use std::sync::Arc;
//! use argo_audit::archive::{archive_window, ArchiveConfig};
//! use argo_store::SessionStore;
//! let store = Arc::new(SessionStore::open(":memory:").await?);
//! store.migrate().await?;
//! let cfg = ArchiveConfig::new(std::path::PathBuf::from("/tmp/argo-archive"));
//! let _bundle = archive_window(&store, &cfg).await?;
//! # Ok(()) }
//! ```
use crate::export::ExportedRow;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Duration, Utc};
use flate2::{write::GzEncoder, Compression};
use fs2::FileExt;
use hmac::{Mac, SimpleHmac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Env var holding the hex-encoded HMAC-SHA256 signing key. Unset or
/// empty → unkeyed HMAC (bundles are still SHA-256 integrity-checked);
/// the runbook flags running production without this set.
pub const HMAC_KEY_ENV: &str = "ARGO_AUDIT_HMAC_KEY";
/// Append-only JSONL manifest filename, one bundle per line.
pub const MANIFEST_FILENAME: &str = "manifest.jsonl";
/// Lock file serializing concurrent runs over the same archive_dir.
pub const LOCK_FILENAME: &str = ".archive.lock";
/// Default online-retention window (days).
pub const DEFAULT_RETENTION_DAYS: u32 = 90;

/// Operator-tunable knobs for one archival run.
#[derive(Debug, Clone)]
pub struct ArchiveConfig {
    /// Rows older than this many days are archive candidates.
    pub retention_days: u32,
    /// Delete archived rows from `audit_record` after the bundle is
    /// durable. Defaults to `false` so operators opt in only after
    /// verifying their bundles.
    pub prune_after_archive: bool,
    /// Where the bundle + manifest land.
    pub archive_dir: PathBuf,
    /// When `Some(d)`, rows newer than `d` days are NEVER pruned,
    /// regardless of `retention_days` — guards against operator
    /// typos like `--retention-days 9` wiping recent evidence.
    pub hard_keep_days: Option<u32>,
}

impl ArchiveConfig {
    /// Build with [`DEFAULT_RETENTION_DAYS`] and pruning disabled.
    #[must_use]
    pub fn new(archive_dir: PathBuf) -> Self {
        Self {
            retention_days: DEFAULT_RETENTION_DAYS,
            prune_after_archive: false,
            archive_dir,
            hard_keep_days: None,
        }
    }
}

/// One archived bundle. Persisted as one line in `manifest.jsonl`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveBundle {
    /// UUIDv7, also embedded in the filename.
    pub id: String,
    /// Inclusive lower bound on archived `created_at`.
    pub range_start: DateTime<Utc>,
    /// Inclusive upper bound on archived `created_at`.
    pub range_end: DateTime<Utc>,
    /// Cutoff timestamp the bundle was generated against — i.e.
    /// `now - retention_days`. Drives day-granularity idempotency.
    pub cutoff: DateTime<Utc>,
    /// Number of audit rows in the bundle (may be 0).
    pub row_count: u64,
    /// SHA-256 of the gzipped tarball file (hex).
    pub sha256: String,
    /// HMAC-SHA256 of the tarball using the chain HMAC key (hex).
    /// Empty key → unkeyed HMAC tag.
    pub signed_by: String,
    /// When the bundle was written.
    pub created_at: DateTime<Utc>,
    /// Bundle filename relative to `archive_dir`.
    pub filename: String,
}

fn pe(ctx: impl std::fmt::Display, e: impl std::fmt::Display) -> ArgoError {
    ArgoError::Provider(format!("{ctx}: {e}"))
}

/// Archive every row whose `created_at` is on or before
/// `now - retention_days` into a fresh bundle. Idempotent within a
/// UTC day: if the manifest already contains a bundle for the same
/// cutoff day, that bundle is returned untouched.
///
/// # Errors
/// Returns [`ArgoError::Provider`] on I/O / SQL / serialization
/// failure.
pub async fn archive_window(
    store: &SessionStore,
    cfg: &ArchiveConfig,
) -> ArgoResult<ArchiveBundle> {
    std::fs::create_dir_all(&cfg.archive_dir)
        .map_err(|e| pe(format!("archive_dir {}", cfg.archive_dir.display()), e))?;
    let _lock = acquire_lock(&cfg.archive_dir)?;
    let now = Utc::now();
    let cutoff = now - Duration::days(i64::from(cfg.retention_days));
    if let Some(existing) = find_idempotent_bundle(&cfg.archive_dir, cutoff, now)? {
        return Ok(existing);
    }
    let rows = fetch_rows_until(store, cutoff).await?;
    let (range_start, range_end) = window_bounds(&rows, cutoff);
    let id = Uuid::now_v7().to_string();
    let filename = format!("{}--{id}.tar.gz", now.format("%Y-%m-%d"));
    let bytes = build_tarball(&rows, range_start, range_end)?;
    write_atomic(&cfg.archive_dir.join(&filename), &bytes)?;
    let bundle = ArchiveBundle {
        id,
        range_start,
        range_end,
        cutoff,
        row_count: rows.len() as u64,
        sha256: sha256_hex(&bytes),
        signed_by: hmac_hex(&load_hmac_key(), &bytes)?,
        created_at: now,
        filename,
    };
    append_manifest(&cfg.archive_dir, &bundle)?;
    if cfg.prune_after_archive {
        prune_archived(store, &rows, cfg, now).await?;
    }
    Ok(bundle)
}

/// Read every entry in `manifest.jsonl`. Empty if absent.
///
/// # Errors
/// Returns [`ArgoError::Provider`] on I/O / parse failure.
pub fn list_archives(archive_dir: &Path) -> ArgoResult<Vec<ArchiveBundle>> {
    let manifest = archive_dir.join(MANIFEST_FILENAME);
    if !manifest.exists() {
        return Ok(Vec::new());
    }
    let body = std::fs::read_to_string(&manifest)
        .map_err(|e| pe(format!("read {}", manifest.display()), e))?;
    let mut out = Vec::new();
    for (i, line) in body.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        out.push(
            serde_json::from_str(line)
                .map_err(|e| pe(format!("manifest.jsonl line {}", i + 1), e))?,
        );
    }
    Ok(out)
}

/// Recompute SHA-256 + HMAC over the on-disk bundle and assert they
/// match the manifest entry.
///
/// # Errors
/// Returns [`ArgoError::Provider`] on integrity failure or I/O.
pub fn verify_archive(bundle: &ArchiveBundle, archive_dir: &Path) -> ArgoResult<()> {
    let path = archive_dir.join(&bundle.filename);
    let mut bytes = Vec::new();
    std::fs::File::open(&path)
        .and_then(|mut f| f.read_to_end(&mut bytes).map(|_| ()))
        .map_err(|e| pe(format!("open {}", path.display()), e))?;
    let actual_sha = sha256_hex(&bytes);
    if actual_sha != bundle.sha256 {
        return Err(pe(
            format!("verify {}", bundle.filename),
            format!("sha256 mismatch (manifest={}, actual={actual_sha})", bundle.sha256),
        ));
    }
    let actual_hmac = hmac_hex(&load_hmac_key(), &bytes)?;
    if actual_hmac != bundle.signed_by {
        return Err(pe(
            format!("verify {}", bundle.filename),
            format!("hmac mismatch (manifest={}, actual={actual_hmac})", bundle.signed_by),
        ));
    }
    Ok(())
}

fn acquire_lock(archive_dir: &Path) -> ArgoResult<std::fs::File> {
    let path = archive_dir.join(LOCK_FILENAME);
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(&path)
        .map_err(|e| pe(format!("lock open {}", path.display()), e))?;
    file.lock_exclusive()
        .map_err(|e| pe(format!("flock {}", path.display()), e))?;
    Ok(file)
}

fn find_idempotent_bundle(
    archive_dir: &Path,
    cutoff: DateTime<Utc>,
    now: DateTime<Utc>,
) -> ArgoResult<Option<ArchiveBundle>> {
    let today = now.date_naive();
    let cutoff_day = cutoff.date_naive();
    Ok(list_archives(archive_dir)?
        .into_iter()
        .find(|b| b.created_at.date_naive() == today && b.cutoff.date_naive() == cutoff_day))
}

async fn fetch_rows_until(
    store: &SessionStore,
    cutoff: DateTime<Utc>,
) -> ArgoResult<Vec<ExportedRow>> {
    let mut rows = Vec::new();
    let cutoff_micros = cutoff.timestamp_micros();
    crate::export::export_ndjson(store, |row| {
        if row.created_at_micros <= cutoff_micros {
            rows.push(row.clone());
        }
        Ok(())
    })
    .await?;
    Ok(rows)
}

fn window_bounds(rows: &[ExportedRow], cutoff: DateTime<Utc>) -> (DateTime<Utc>, DateTime<Utc>) {
    if rows.is_empty() {
        return (cutoff, cutoff);
    }
    let lo = rows.iter().map(|r| r.created_at_micros).min().unwrap_or(0);
    let hi = rows.iter().map(|r| r.created_at_micros).max().unwrap_or(0);
    (
        DateTime::<Utc>::from_timestamp_micros(lo).unwrap_or(cutoff),
        DateTime::<Utc>::from_timestamp_micros(hi).unwrap_or(cutoff),
    )
}

fn build_tarball(
    rows: &[ExportedRow],
    range_start: DateTime<Utc>,
    range_end: DateTime<Utc>,
) -> ArgoResult<Vec<u8>> {
    let mut ndjson = Vec::new();
    for row in rows {
        let line = serde_json::to_string(row).map_err(|e| pe("serialize row", e))?;
        ndjson.extend_from_slice(line.as_bytes());
        ndjson.push(b'\n');
    }
    let metadata = serde_json::to_vec_pretty(&serde_json::json!({
        "generator": "argo audit archive",
        "generator_version": env!("CARGO_PKG_VERSION"),
        "row_count": rows.len(),
        "range_start": range_start.to_rfc3339(),
        "range_end": range_end.to_rfc3339(),
        "audit_ndjson_sha256": sha256_hex(&ndjson),
    }))
    .map_err(|e| pe("serialize chain_metadata", e))?;
    let mut gz = GzEncoder::new(Vec::new(), Compression::default());
    {
        let mut tar_b = tar::Builder::new(&mut gz);
        append_tar(&mut tar_b, "audit.ndjson", &ndjson)?;
        append_tar(&mut tar_b, "chain_metadata.json", &metadata)?;
        tar_b.finish().map_err(|e| pe("tar finish", e))?;
    }
    gz.finish().map_err(|e| pe("gzip finish", e))
}

fn append_tar<W: Write>(b: &mut tar::Builder<W>, name: &str, body: &[u8]) -> ArgoResult<()> {
    let mut header = tar::Header::new_gnu();
    header.set_size(body.len() as u64);
    header.set_mode(0o644);
    header.set_mtime(0);
    header.set_cksum();
    b.append_data(&mut header, name, body)
        .map_err(|e| pe(format!("append {name}"), e))
}

fn write_atomic(path: &Path, bytes: &[u8]) -> ArgoResult<()> {
    let tmp = path.with_extension("tar.gz.tmp");
    std::fs::write(&tmp, bytes).map_err(|e| pe(format!("write {}", tmp.display()), e))?;
    std::fs::rename(&tmp, path).map_err(|e| pe(format!("rename {}", path.display()), e))
}

fn append_manifest(archive_dir: &Path, bundle: &ArchiveBundle) -> ArgoResult<()> {
    let path = archive_dir.join(MANIFEST_FILENAME);
    let line = serde_json::to_string(bundle).map_err(|e| pe("serialize manifest", e))?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| pe(format!("open {}", path.display()), e))?;
    file.write_all(line.as_bytes())
        .and_then(|()| file.write_all(b"\n"))
        .map_err(|e| pe(format!("write {}", path.display()), e))
}

async fn prune_archived(
    store: &SessionStore,
    rows: &[ExportedRow],
    cfg: &ArchiveConfig,
    now: DateTime<Utc>,
) -> ArgoResult<()> {
    let hard_floor = cfg
        .hard_keep_days
        .map(|d| (now - Duration::days(i64::from(d))).timestamp_micros());
    for row in rows {
        if let Some(floor) = hard_floor {
            if row.created_at_micros > floor {
                continue;
            }
        }
        let id_bytes = hex::decode(&row.id).map_err(|e| pe(format!("hex id {}", row.id), e))?;
        sqlx::query("DELETE FROM audit_record WHERE id = ?")
            .bind(id_bytes)
            .execute(store.pool())
            .await
            .map_err(|e| pe(format!("prune row {}", row.id), e))?;
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

fn hmac_hex(key: &[u8], bytes: &[u8]) -> ArgoResult<String> {
    let mut mac = <SimpleHmac<Sha256> as Mac>::new_from_slice(key).map_err(|e| pe("hmac key", e))?;
    mac.update(bytes);
    Ok(hex::encode(mac.finalize().into_bytes()))
}

fn load_hmac_key() -> Vec<u8> {
    std::env::var(HMAC_KEY_ENV)
        .ok()
        .and_then(|raw| hex::decode(raw.trim()).ok())
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "archive_tests.rs"]
mod tests;
