//! `GET /verticals` + `GET /verticals/<id>` — read-only discovery
//! surface for the vertical-template system (#650).
//!
//! Loads on every request via `argo_vertical::load_dir`. That's
//! cheap for the small N expected today; once the catalogue is
//! larger or hot-paths matter, swap in a cached `Arc<Registry>`.
//! Discovery directory is shared with the CLI handler — same env
//! var (`ARGO_VERTICALS_DIR`) so test setup + production runs stay
//! consistent across both surfaces.
//!
//! Out of scope (deferred per #650's plan): dynamic mounting of
//! per-vertical `[gateway].public_endpoints` — that's Phase 4 (or
//! later) once we know what shape the verticals actually want to
//! expose.
use argo_vertical::load_dir;

/// `GET /verticals[?format=markdown]` — list every loaded vertical
/// plus any per-vertical load errors. JSON shape:
/// `{ok, loaded:[{id,name,version,...}], errors:[{dir,error}]}`.
pub(super) async fn list_response(format: Option<&str>) -> (&'static str, String) {
    let dir = crate::handlers::vertical::resolve_verticals_dir();
    let (registry, errors) = match load_dir(&dir) {
        Ok(r) => r,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"verticals load_dir: {e}\"}}"),
            );
        }
    };
    if matches!(format, Some("markdown")) {
        let mut out = format!(
            "# Verticals ({} loaded, {} error{})\n\n",
            registry.len(),
            errors.len(),
            if errors.len() == 1 { "" } else { "s" }
        );
        if registry.is_empty() && errors.is_empty() {
            out.push_str(&format!(
                "No verticals discovered in `{}` (override with `ARGO_VERTICALS_DIR`).\n",
                dir.display()
            ));
        }
        if !registry.is_empty() {
            out.push_str("| id | name | version | description |\n");
            out.push_str("| --- | --- | --- | --- |\n");
            for (id, v) in registry.list() {
                let desc = v.manifest.vertical.description.replace('|', "\\|");
                out.push_str(&format!(
                    "| `{id}` | {} | {} | {desc} |\n",
                    v.manifest.vertical.name, v.manifest.vertical.version
                ));
            }
        }
        if !errors.is_empty() {
            out.push_str("\n## Errors\n\n");
            for e in &errors {
                out.push_str(&format!("- `{}`: {}\n", e.dir.display(), e.error));
            }
        }
        return ("200 OK", out);
    }
    let arr: Vec<serde_json::Value> = registry
        .list()
        .map(|(id, v)| {
            serde_json::json!({
                "id": id,
                "name": v.manifest.vertical.name,
                "version": v.manifest.vertical.version,
                "description": v.manifest.vertical.description,
                "tags": v.manifest.vertical.tags,
                "root": v.root.display().to_string(),
            })
        })
        .collect();
    let errs: Vec<serde_json::Value> = errors
        .iter()
        .map(|e| {
            serde_json::json!({
                "dir": e.dir.display().to_string(),
                "error": format!("{}", e.error),
            })
        })
        .collect();
    (
        "200 OK",
        serde_json::to_string(&serde_json::json!({
            "ok": errors.is_empty(),
            "loaded": arr,
            "errors": errs,
        }))
        .unwrap_or_else(|_| "{}".to_string()),
    )
}

/// `GET /verticals/<id>[?format=markdown]` — full manifest summary
/// for one vertical. 404 + `{"error":"…"}` if not found. Reuses
/// the CLI's `summarize_loaded` + `render_markdown` so the wire
/// shape matches `argo vertical show --json` byte-for-byte.
pub(super) async fn show_response(
    id: &str,
    format: Option<&str>,
) -> (&'static str, String) {
    let dir = crate::handlers::vertical::resolve_verticals_dir();
    let (registry, _errors) = match load_dir(&dir) {
        Ok(r) => r,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"verticals load_dir: {e}\"}}"),
            );
        }
    };
    let Some(loaded) = registry.get(id) else {
        return (
            "404 Not Found",
            format!("{{\"error\":\"vertical '{id}' not found\"}}"),
        );
    };
    if matches!(format, Some("markdown")) {
        return ("200 OK", crate::handlers::vertical::render_markdown(loaded));
    }
    (
        "200 OK",
        serde_json::to_string(&crate::handlers::vertical::summarize_loaded(loaded))
            .unwrap_or_else(|_| "{}".to_string()),
    )
}
