//! `GET /verticals` + `GET /verticals/<id>` — read-only discovery
//! surface for the vertical-template system (#650, extended for
//! `mission-vertical-template-marketplace-listing`).
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
//!
//! Marketplace-listing additions (additive, no breaking changes):
//! - `list_response` rows carry `headline`, `workflow_count`,
//!   `action_count`, and `role_label` so the `/verticals` web
//!   marketplace card UI can render without a second per-vertical
//!   fetch.
//! - `show_response` returns the manifest summary unchanged plus a
//!   `marketplace` envelope with `workflows[]`, `actions[]` (name +
//!   description + risk), and `success_metrics` — the data the
//!   detail page renders under "what this template ships with."
//!
//! `skill_count` (added for the onboarding pick-vertical wizard) is
//! computed from on-disk `<root>/skills/*.skill.md` files so it
//! matches `skills_added` returned by
//! `POST /api/v1/onboarding/install-vertical`. The wizard renders
//! this number on each card so the count never drifts from what the
//! gateway will actually install.
use argo_vertical::{load_dir, LoadedVertical};
use std::path::Path;

/// `GET /verticals[?format=markdown]` — list every loaded vertical
/// plus any per-vertical load errors. JSON shape:
/// `{ok, loaded:[{id,name,version,headline,workflow_count,action_count,role_label,...}], errors:[{dir,error}]}`.
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
    let arr: Vec<serde_json::Value> = registry.list().map(|(id, v)| list_row(id, v)).collect();
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
/// shape matches `argo vertical show --json` byte-for-byte; an
/// additional `marketplace` envelope is appended additively for
/// the `/verticals/<id>` detail UI.
pub(super) async fn show_response(id: &str, format: Option<&str>) -> (&'static str, String) {
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
    let mut payload = crate::handlers::vertical::summarize_loaded(loaded);
    if let Some(obj) = payload.as_object_mut() {
        obj.insert("headline".to_string(), serde_json::Value::String(headline(loaded)));
        obj.insert("marketplace".to_string(), marketplace_envelope(loaded));
    }
    (
        "200 OK",
        serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()),
    )
}

/// Project one vertical into the marketplace-listing row shape:
/// the CLI's existing fields (`id`/`name`/`version`/`description`/
/// `tags`/`root`) plus card-render fields the listing UI needs.
fn list_row(id: &str, v: &LoadedVertical) -> serde_json::Value {
    let m = &v.manifest;
    serde_json::json!({
        "id": id,
        "name": m.vertical.name,
        "version": m.vertical.version,
        "description": m.vertical.description,
        "headline": headline(v),
        "tags": m.vertical.tags,
        "root": v.root.display().to_string(),
        "workflow_count": m.workflows.as_ref().map(|w| w.files.len()).unwrap_or(0),
        "action_count": m.actions.as_ref().map(|a| a.actions.len()).unwrap_or(0),
        "role_label": role_label(v),
        "kpi_count": m.all_kpis().len(),
        "skill_count": skill_count(v),
    })
}

/// Count `*.skill.md` files in `<root>/skills/`. Mirrors the helper
/// at `routes_onboarding_helpers::count_skill_files` so the number
/// the wizard renders on each card equals the `skills_added`
/// returned by `POST /api/v1/onboarding/install-vertical`. Returns 0
/// when the directory is missing or unreadable.
fn skill_count(v: &LoadedVertical) -> u64 {
    let dir = v.root.join("skills");
    count_files_with_suffix(&dir, "skill.md")
}

fn count_files_with_suffix(dir: &Path, suffix: &str) -> u64 {
    let Ok(read) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut count: u64 = 0;
    for entry in read.flatten() {
        if entry
            .file_name()
            .to_string_lossy()
            .ends_with(&format!(".{suffix}"))
        {
            count += 1;
        }
    }
    count
}

/// One-sentence headline pulled from `vertical.description`. We
/// keep the first sentence (terminated by `.`, `?`, or `!`) so the
/// card stays compact; the full description stays available on the
/// detail page. Falls back to the full description when the first
/// sentence isn't bounded.
fn headline(v: &LoadedVertical) -> String {
    let desc = v.manifest.vertical.description.trim();
    if desc.is_empty() {
        return String::new();
    }
    let end = desc
        .char_indices()
        .find(|(_, c)| matches!(c, '.' | '?' | '!'))
        .map(|(i, c)| i + c.len_utf8());
    match end {
        Some(idx) => desc[..idx].to_string(),
        None => desc.to_string(),
    }
}

/// Human-readable label for the vertical's role. v1 picks the
/// vertical's display name — every loaded vertical is itself a
/// role template, and the manifest's `[role]` section only points
/// at file paths (not a label). Operator-friendly default.
fn role_label(v: &LoadedVertical) -> String {
    v.manifest.vertical.name.clone()
}

/// Card-detail envelope appended additively to `summarize_loaded`'s
/// output. Holds the data `/verticals/<id>` renders under the
/// "what this template ships with" panels.
fn marketplace_envelope(v: &LoadedVertical) -> serde_json::Value {
    let m = &v.manifest;
    let workflows: Vec<serde_json::Value> = m
        .workflows
        .as_ref()
        .map(|w| {
            w.files
                .iter()
                .map(|f| serde_json::json!({ "file": f }))
                .collect()
        })
        .unwrap_or_default();
    let actions: Vec<serde_json::Value> = m
        .actions
        .as_ref()
        .map(|a| {
            a.actions
                .iter()
                .map(|(name, decl)| {
                    serde_json::json!({
                        "name": name,
                        "description": decl.description,
                        "risk": format!("{:?}", decl.risk).to_lowercase(),
                        "requires_approval": decl.requires_approval,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let kpis: Vec<serde_json::Value> = m
        .all_kpis()
        .iter()
        .map(|k| {
            serde_json::json!({
                "name": k.name,
                "kind": format!("{:?}", k.kind).to_lowercase(),
                "threshold": k.threshold,
                "comparator": format!("{:?}", k.comparator).to_lowercase(),
                "gate": format!("{:?}", k.gate).to_lowercase(),
            })
        })
        .collect();
    serde_json::json!({
        "headline": headline(v),
        "role_label": role_label(v),
        "workflows": workflows,
        "workflow_count": workflows.len(),
        "actions": actions,
        "action_count": actions.len(),
        "kpis": kpis,
        "tags": m.vertical.tags,
    })
}
