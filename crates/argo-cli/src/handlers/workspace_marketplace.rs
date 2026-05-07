//! Marketplace-flavoured `argo workspace` subcommands —
//! push / pull / fork / diff / artifacts. Lives in its own
//! module so `workspace.rs` stays under the 500-LOC ceiling
//! as the surface grows.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_marketplace::{
    build_bundle, OciDistributionRegistry, OciReference, OciRegistry,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use std::path::PathBuf;

pub(crate) async fn push(cwd: &std::path::Path, oci_ref: &str) -> ArgoResult<()> {
    let reference = OciReference::parse(oci_ref)?;
    let out_tar: PathBuf = std::env::temp_dir().join(format!(
        "argo-workspace-{}-{}.tar",
        reference.repo.replace('/', "-"),
        reference.tag
    ));
    let manifest = build_bundle(cwd, &reference.repo, &reference.tag, &out_tar).await?;
    println!("built bundle: {}", out_tar.display());
    println!("  workspace: {}", manifest.workspace_name);
    println!("  tag:       {}", manifest.tag);
    println!("  digest:    sha256:{}", manifest.digest);
    println!("  entries:   {}", manifest.entries.len());

    let tar_bytes = tokio::fs::read(&out_tar)
        .await
        .map_err(|e| ArgoError::Provider(format!("read {}: {e}", out_tar.display())))?;
    let registry = build_registry(&reference.host);
    registry.push(&reference, &tar_bytes, &manifest).await?;
    println!(
        "pushed {} ({} bytes) → {}",
        reference.to_canonical(),
        tar_bytes.len(),
        reference.host
    );

    if let Err(e) = record_artifact(cwd, &reference, &manifest.digest).await {
        eprintln!("warning: marketplace_artifact insert failed: {e}");
    }
    Ok(())
}

pub(crate) async fn pull(cwd: &std::path::Path, oci_ref: &str) -> ArgoResult<()> {
    let reference = OciReference::parse(oci_ref)?;
    let registry = build_registry(&reference.host);
    let (tar_bytes, manifest) = registry.pull(&reference).await?;
    let out_tar = cwd.join(format!(
        "argo-pull-{}-{}.tar",
        reference.repo.replace('/', "-"),
        reference.tag
    ));
    tokio::fs::write(&out_tar, &tar_bytes)
        .await
        .map_err(|e| ArgoError::Provider(format!("write {}: {e}", out_tar.display())))?;
    println!(
        "pulled {} → {} ({} bytes)",
        reference.to_canonical(),
        out_tar.display(),
        tar_bytes.len()
    );
    println!("  workspace: {}", manifest.workspace_name);
    println!("  tag:       {}", manifest.tag);
    println!("  digest:    {}", manifest.digest);
    println!("  entries:   {}", manifest.entries.len());
    Ok(())
}

/// `argo workspace fork <oci_ref> [--into <dir>]` — pull a
/// bundle from a registry and extract it into a fresh
/// directory. Refuses to clobber an existing target.
pub(crate) async fn fork(
    cwd: &std::path::Path,
    oci_ref: &str,
    into: Option<&str>,
) -> ArgoResult<()> {
    let reference = OciReference::parse(oci_ref)?;
    let dest_name = into.map(String::from).unwrap_or_else(|| {
        format!(
            "fork-{}-{}",
            reference.repo.replace('/', "-"),
            reference.tag
        )
    });
    let dest = if std::path::Path::new(&dest_name).is_absolute() {
        std::path::PathBuf::from(&dest_name)
    } else {
        cwd.join(&dest_name)
    };
    if dest.exists() {
        return Err(ArgoError::Provider(format!(
            "workspace fork: destination '{}' already exists; refusing to overwrite",
            dest.display()
        )));
    }
    let registry = build_registry(&reference.host);
    let (tar_bytes, manifest) = registry.pull(&reference).await?;
    std::fs::create_dir_all(&dest)
        .map_err(|e| ArgoError::Provider(format!("mkdir {}: {e}", dest.display())))?;
    let mut archive = tar::Archive::new(&tar_bytes[..]);
    archive
        .unpack(&dest)
        .map_err(|e| ArgoError::Provider(format!("untar into {}: {e}", dest.display())))?;
    println!(
        "forked {} → {} ({} entries, digest {})",
        reference.to_canonical(),
        dest.display(),
        manifest.entries.len(),
        manifest.digest
    );
    println!("  cd {} && argo workspace verify", dest.display());
    Ok(())
}

/// Diff the cwd's workspace bundle against another local path.
pub(crate) async fn diff(cwd: &std::path::Path, other_raw: &str) -> ArgoResult<()> {
    let other = std::path::PathBuf::from(other_raw);
    if !other.exists() {
        return Err(ArgoError::Provider(format!(
            "workspace diff: path '{other_raw}' does not exist"
        )));
    }
    let tmp = std::env::temp_dir();
    let tar_a = tmp.join(format!("argo-diff-a-{}.tar", std::process::id()));
    let tar_b = tmp.join(format!("argo-diff-b-{}.tar", std::process::id()));
    let manifest_a = build_bundle(cwd, "diff-a", "0", &tar_a).await?;
    let manifest_b = build_bundle(&other, "diff-b", "0", &tar_b).await?;
    let _ = std::fs::remove_file(&tar_a);
    let _ = std::fs::remove_file(&tar_b);

    let mut a_by_path: std::collections::HashMap<&str, &str> = manifest_a
        .entries
        .iter()
        .map(|e| (e.path.as_str(), e.digest.as_str()))
        .collect();
    let mut added: Vec<&str> = Vec::new();
    let mut modified: Vec<(&str, &str, &str)> = Vec::new();
    for e in &manifest_b.entries {
        match a_by_path.remove(e.path.as_str()) {
            None => added.push(e.path.as_str()),
            Some(d_a) if d_a != e.digest => {
                modified.push((e.path.as_str(), d_a, e.digest.as_str()));
            }
            Some(_) => {}
        }
    }
    let mut removed: Vec<&str> = a_by_path.into_keys().collect();
    removed.sort();
    added.sort();
    modified.sort_by_key(|(p, _, _)| *p);

    if added.is_empty() && removed.is_empty() && modified.is_empty() {
        println!(
            "workspace diff: {} and {} are identical (digest sha256:{})",
            cwd.display(),
            other.display(),
            manifest_a.digest
        );
        return Ok(());
    }
    println!("workspace diff: {} → {}", cwd.display(), other.display());
    for p in &added {
        println!("  + {p}");
    }
    for p in &removed {
        println!("  - {p}");
    }
    for (p, a, b) in &modified {
        let a_short: String = a.chars().take(12).collect();
        let b_short: String = b.chars().take(12).collect();
        println!("  ~ {p}  ({a_short} → {b_short})");
    }
    println!(
        "\nsummary: {} added, {} removed, {} modified",
        added.len(),
        removed.len(),
        modified.len()
    );
    Ok(())
}

pub(crate) async fn artifacts(
    cwd: &std::path::Path,
    limit: u32,
    json: bool,
) -> ArgoResult<()> {
    let home = resolve_home(cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (_tenant, workspace) = store.ensure_workspace(name, &root).await?;
    let rows = store.list_marketplace_artifacts(workspace, limit).await?;
    if json {
        for r in &rows {
            let line = serde_json::to_string(r).unwrap_or_else(|_| "{}".into());
            println!("{line}");
        }
        return Ok(());
    }
    if rows.is_empty() {
        println!("(no marketplace pushes recorded for {})", cwd.display());
        return Ok(());
    }
    println!(
        "{:<48}  {:<24}  {:<32}  by",
        "oci_ref", "pushed_at (utc)", "digest"
    );
    for r in rows {
        let dt = chrono::DateTime::<chrono::Utc>::from_timestamp_micros(r.pushed_at)
            .map(|d| d.format("%Y-%m-%dT%H:%M:%SZ").to_string())
            .unwrap_or_else(|| "?".into());
        let digest_short: String = r.digest.chars().take(28).collect();
        println!(
            "{:<48}  {:<24}  {:<32}  {}",
            r.oci_ref, dt, digest_short, r.pushed_by
        );
    }
    Ok(())
}

async fn record_artifact(
    cwd: &std::path::Path,
    reference: &OciReference,
    digest: &str,
) -> ArgoResult<()> {
    let home = resolve_home(cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (_tenant, workspace) = store.ensure_workspace(name, &root).await?;
    let by = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .map(|u| format!("cli:{u}"))
        .unwrap_or_else(|_| "cli".into());
    store
        .record_marketplace_artifact(workspace, &reference.to_canonical(), digest, &by)
        .await?;
    Ok(())
}

/// Determine the registry's HTTP base URL from the OCI host
/// (e.g. `ghcr.io` → `https://ghcr.io`; `localhost:5000` →
/// `http://localhost:5000` — useful for local dev registries
/// + the smoke-test mock).
fn host_base_url(host: &str) -> String {
    let scheme = if host.starts_with("localhost") || host.starts_with("127.0.0.1") {
        "http"
    } else {
        "https"
    };
    format!("{scheme}://{host}")
}

/// Read `ARGO_OCI_TOKEN` if set; sent as `Authorization: Bearer …`
/// on every registry request, bypassing the challenge handshake.
fn oci_token() -> Option<String> {
    argo_config::cred::get("ARGO_OCI_TOKEN").ok()
}

/// Build a registry from the host with optional Basic-auth
/// credentials read from `ARGO_OCI_USER` + `ARGO_OCI_PASSWORD`
/// (used in the `WWW-Authenticate: Bearer` challenge handshake
/// — GHCR + Docker Hub style). The static `ARGO_OCI_TOKEN`
/// path takes precedence and skips the handshake entirely.
fn build_registry(host: &str) -> OciDistributionRegistry {
    let mut reg = OciDistributionRegistry::new(&host_base_url(host), oci_token());
    if let (Ok(user), Ok(password)) = (
        argo_config::cred::get("ARGO_OCI_USER"),
        argo_config::cred::get("ARGO_OCI_PASSWORD"),
    ) {
        reg = reg.with_basic_auth(user, password);
    }
    reg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_base_url_chooses_https_for_remote_hosts() {
        assert_eq!(host_base_url("ghcr.io"), "https://ghcr.io");
        assert_eq!(host_base_url("registry-1.docker.io"), "https://registry-1.docker.io");
        assert_eq!(host_base_url("registry.gitlab.com"), "https://registry.gitlab.com");
    }

    #[test]
    fn host_base_url_chooses_http_for_local_hosts() {
        assert_eq!(host_base_url("localhost:5000"), "http://localhost:5000");
        assert_eq!(host_base_url("127.0.0.1:5000"), "http://127.0.0.1:5000");
        assert_eq!(host_base_url("localhost"), "http://localhost");
    }

    /// `ARGO_OCI_TOKEN` reads cleanly when set; absent → `None`.
    /// (The actual flow path through `OciDistributionRegistry` is
    /// covered by `argo-marketplace` integration tests; this just
    /// pins the env-var contract.)
    #[test]
    fn oci_token_reads_from_env() {
        // Save + restore the var so concurrent tests don't see
        // each other's mutations.
        let saved = std::env::var("ARGO_OCI_TOKEN").ok();
        std::env::remove_var("ARGO_OCI_TOKEN");
        assert!(oci_token().is_none(), "expected None when env unset");
        std::env::set_var("ARGO_OCI_TOKEN", "ghp_test_token");
        assert_eq!(oci_token().as_deref(), Some("ghp_test_token"));
        match saved {
            Some(s) => std::env::set_var("ARGO_OCI_TOKEN", s),
            None => std::env::remove_var("ARGO_OCI_TOKEN"),
        }
    }
}
