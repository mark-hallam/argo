//! `GET /api/v1/packs` — read-only vertical-pack catalog for the SPA.
//!
//! Bearer auth is enforced upstream by `routes::dispatch_route`, matching
//! other `/api/v1/*` GET surfaces. This handler only merges the repo pack
//! inventory with the operator's local `~/.argo/packs` cache state.

use argo_types::{ArgoError, ArgoResult};
use argo_vertical_packs::{PackLoader, PackLoaderConfig, PackManifest};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const DEFAULT_LIMIT: usize = 50;
const MAX_LIMIT: usize = 100;
const DEFAULT_AUTHOR: &str = "skycot";

#[derive(Debug, Clone, Serialize)]
struct PackEntry {
    id: String,
    version: String,
    vertical_id: String,
    author: String,
    description: String,
    installed: bool,
    source: &'static str,
}

#[derive(Debug, Serialize)]
struct PackPage {
    packs: Vec<PackEntry>,
    pagination: Pagination,
}

#[derive(Debug, Serialize)]
struct Pagination {
    total: usize,
    limit: usize,
    offset: usize,
    has_more: bool,
}

#[derive(Debug, Clone, Copy)]
struct PageRequest {
    limit: usize,
    offset: usize,
}

/// Return the pack catalog as a JSON envelope.
pub(super) fn list_response(req: &str) -> (&'static str, String) {
    let repo = workspace_root().join("packs");
    let cache = match default_cache_root() {
        Ok(path) => path,
        Err(e) => return server_error(&format!("resolve pack cache: {e}")),
    };
    list_response_with_roots(req, &repo, &cache)
}

fn list_response_with_roots(
    req: &str,
    repo_packs_dir: &Path,
    cache_root: &Path,
) -> (&'static str, String) {
    let page = match parse_page(req) {
        Ok(page) => page,
        Err(msg) => return bad_request(&msg),
    };
    let rows = match collect_catalog(repo_packs_dir, cache_root) {
        Ok(rows) => rows,
        Err(e) => return server_error(&format!("list packs: {e}")),
    };
    let total = rows.len();
    let packs: Vec<PackEntry> = rows
        .into_iter()
        .skip(page.offset)
        .take(page.limit)
        .collect();
    let body = PackPage {
        pagination: Pagination {
            total,
            limit: page.limit,
            offset: page.offset,
            has_more: page.offset.saturating_add(packs.len()) < total,
        },
        packs,
    };
    json_response(&body)
}

fn collect_catalog(repo_packs_dir: &Path, cache_root: &Path) -> ArgoResult<Vec<PackEntry>> {
    let installed = PackLoader::new(PackLoaderConfig::with_cache_dir(cache_root.to_path_buf()))
        .list()
        .map_err(|e| ArgoError::Io(format!("read installed packs: {e}")))?;
    let installed_keys: BTreeSet<(String, String)> = installed
        .iter()
        .map(|row| (row.name.clone(), row.version.clone()))
        .collect();
    let mut rows: BTreeMap<(String, String), PackEntry> = BTreeMap::new();

    for root in repo_pack_roots(repo_packs_dir)? {
        let manifest = PackManifest::load(&root)?;
        let key = (manifest.pack.name.clone(), manifest.pack.version.clone());
        let installed = installed_keys.contains(&key);
        rows.insert(key, entry_from_manifest(manifest, installed, "repo"));
    }

    for row in installed {
        let manifest = PackManifest::load(&row.root)?;
        let key = (manifest.pack.name.clone(), manifest.pack.version.clone());
        rows.entry(key)
            .and_modify(|entry| entry.installed = true)
            .or_insert_with(|| entry_from_manifest(manifest, true, "cache"));
    }

    Ok(rows.into_values().collect())
}

fn repo_pack_roots(packs_dir: &Path) -> ArgoResult<Vec<PathBuf>> {
    let mut roots = Vec::new();
    if !packs_dir.is_dir() {
        return Ok(roots);
    }
    for entry in std::fs::read_dir(packs_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() && entry.path().join("pack.toml").is_file() {
            roots.push(entry.path());
        }
    }
    roots.sort();
    Ok(roots)
}

fn entry_from_manifest(manifest: PackManifest, installed: bool, source: &'static str) -> PackEntry {
    let vertical_id = manifest
        .verticals
        .first()
        .map(|v| v.id.clone())
        .unwrap_or_default();
    PackEntry {
        id: manifest.pack.name,
        version: manifest.pack.version,
        vertical_id,
        author: manifest
            .pack
            .author
            .unwrap_or_else(|| DEFAULT_AUTHOR.to_string()),
        description: manifest.pack.description.unwrap_or_default(),
        installed,
        source,
    }
}

fn parse_page(req: &str) -> Result<PageRequest, String> {
    let query = req
        .strip_prefix("GET ")
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|target| target.split_once('?').map(|(_, query)| query))
        .unwrap_or("");
    let limit = parse_usize_param(query, "limit")?.unwrap_or(DEFAULT_LIMIT);
    let offset = parse_usize_param(query, "offset")?.unwrap_or(0);
    Ok(PageRequest {
        limit: limit.clamp(1, MAX_LIMIT),
        offset,
    })
}

fn parse_usize_param(query: &str, key: &str) -> Result<Option<usize>, String> {
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        if k == key {
            return v
                .parse::<usize>()
                .map(Some)
                .map_err(|e| format!("invalid {key}: {e}"));
        }
    }
    Ok(None)
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn default_cache_root() -> ArgoResult<PathBuf> {
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| ArgoError::Io("HOME is not set; cannot resolve ~/.argo/packs".into()))?;
    Ok(home.join(".argo").join("packs"))
}

fn json_response<T: Serialize>(value: &T) -> (&'static str, String) {
    match serde_json::to_string(value) {
        Ok(body) => ("200 OK", body),
        Err(e) => server_error(&format!("serialize packs response: {e}")),
    }
}

fn bad_request(message: &str) -> (&'static str, String) {
    error_response("400 Bad Request", "bad_request", message)
}

fn server_error(message: &str) -> (&'static str, String) {
    error_response("500 Internal Server Error", "internal", message)
}

fn error_response(status: &'static str, code: &str, message: &str) -> (&'static str, String) {
    (
        status,
        serde_json::json!({
            "error": {
                "code": code,
                "message": message,
            }
        })
        .to_string(),
    )
}

#[cfg(test)]
#[path = "routes_packs_tests.rs"]
mod tests;
