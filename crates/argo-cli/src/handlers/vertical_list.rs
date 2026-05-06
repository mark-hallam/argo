use super::vertical::resolve_verticals_dir;
use argo_types::{ArgoError, ArgoResult};
use argo_vertical::{load_dir, LoadedVertical};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

const NO_LIBRARY: &str = "no library configured (library vertical catalogue is planned for v1.x)";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Source {
    Workspace,
    Library,
    Both,
}

#[derive(Debug, Serialize)]
struct ListOutput<'a> {
    ok: bool,
    tenant: Option<&'a str>,
    source: &'static str,
    include_archived: bool,
    loaded: Vec<ListRow>,
    errors: Vec<ListError>,
    messages: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ListRow {
    id: String,
    version: String,
    persona: String,
    action_count: usize,
    source: &'static str,
    last_modified: Option<String>,
    archived_at: Option<i64>,
    root: String,
}

#[derive(Debug, Serialize)]
struct ListError {
    dir: String,
    error: String,
}

pub async fn run(
    tenant: Option<&str>,
    source: &str,
    include_archived: bool,
    json: bool,
) -> ArgoResult<()> {
    let source = parse_source(source)?;
    let mut rows = Vec::new();
    let mut errors = Vec::new();
    let mut messages = Vec::new();
    let workspace_dir = resolve_verticals_dir();
    let archived = if matches!(source, Source::Workspace | Source::Both) {
        super::vertical_archive::archived_map_for_default_store().await?
    } else {
        BTreeMap::new()
    };

    if matches!(source, Source::Workspace | Source::Both) {
        let (registry, load_errors) = load_dir(&workspace_dir)?;
        rows.extend(registry.list().filter_map(|(id, v)| {
            let archived_at = archived.get(id).copied();
            if archived_at.is_some() && !include_archived {
                None
            } else {
                Some(list_row(id, v, archived_at))
            }
        }));
        errors.extend(load_errors.into_iter().map(|e| ListError {
            dir: e.dir.display().to_string(),
            error: e.error.to_string(),
        }));
    }

    if matches!(source, Source::Library | Source::Both) {
        messages.push(NO_LIBRARY.to_string());
    }

    if json {
        print_json(&ListOutput {
            ok: errors.is_empty(),
            tenant,
            source: source.as_str(),
            include_archived,
            loaded: rows,
            errors,
            messages,
        });
        return Ok(());
    }

    if let Some(tenant) = tenant {
        println!("tenant: {tenant}");
    }

    if rows.is_empty() && errors.is_empty() {
        if matches!(source, Source::Workspace | Source::Both) {
            println!("(no verticals in {})", workspace_dir.display());
        }
    } else {
        println!(
            "{:<24} {:<10} {:<22} {:>7} {:<9} last_modified",
            "id", "version", "persona", "actions", "source"
        );
        for row in &rows {
            println!(
                "{:<24} {:<10} {:<22} {:>7} {:<9} {}",
                row.id,
                row.version,
                row.persona,
                row.action_count,
                row.source,
                row.last_modified.as_deref().unwrap_or("-")
            );
        }
    }

    for msg in &messages {
        println!("{msg}");
    }
    if !errors.is_empty() {
        println!("errors:");
        for err in &errors {
            println!("  {}: {}", err.dir, err.error);
        }
    }
    Ok(())
}

fn parse_source(raw: &str) -> ArgoResult<Source> {
    match raw {
        "workspace" => Ok(Source::Workspace),
        "library" => Ok(Source::Library),
        "both" => Ok(Source::Both),
        other => Err(ArgoError::Provider(format!(
            "unknown vertical source '{other}' (expected workspace, library, or both)"
        ))),
    }
}

impl Source {
    fn as_str(self) -> &'static str {
        match self {
            Source::Workspace => "workspace",
            Source::Library => "library",
            Source::Both => "both",
        }
    }
}

fn list_row(id: &str, v: &LoadedVertical, archived_at: Option<i64>) -> ListRow {
    let m = &v.manifest;
    ListRow {
        id: id.to_string(),
        version: m.vertical.version.clone(),
        persona: m
            .role
            .as_ref()
            .and_then(|r| r.persona.clone())
            .unwrap_or_else(|| "-".to_string()),
        action_count: m.actions.as_ref().map_or(0, |a| a.actions.len()),
        source: "workspace",
        last_modified: last_modified(&v.root),
        archived_at,
        root: v.root.display().to_string(),
    }
}

fn last_modified(path: &Path) -> Option<String> {
    let modified = std::fs::metadata(path).and_then(|m| m.modified()).ok()?;
    let dt: DateTime<Utc> = modified.into();
    Some(dt.to_rfc3339_opts(SecondsFormat::Secs, true))
}

fn print_json<T: Serialize>(value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(body) => println!("{body}"),
        Err(e) => println!(r#"{{"error":"json encode: {e}"}}"#),
    }
}
