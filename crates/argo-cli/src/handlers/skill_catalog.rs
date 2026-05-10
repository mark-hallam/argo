//! `argo skill catalog` — browse marketplace skills with local-install marks.
//!
//! Unlike `skill list` (local installed skills) and `skill search`
//! (query-driven marketplace search), `skill catalog` walks the full
//! marketplace `MANIFEST_INDEX` and joins it against the locally-
//! installed set to mark each entry `[x]` installed or `[ ]` not.
//!
//! Marketplace schema gap (sensible deviation): the current
//! `IndexedSkill` struct does not carry separate `vertical` or `tags`
//! fields. We surface the closest analogs — the publisher `tenant`
//! stands in for the vertical id, and `--tag` is a case-insensitive
//! substring filter over the entry description. Tags are reported as
//! an empty list in JSON output until the upstream schema grows real
//! tag columns (separate brief, owns `IndexedSkill`).

use argo_skill::marketplace::{search, MarketplaceSkill, SearchOptions};
use argo_skill::SkillStore;
use argo_types::{ArgoError, ArgoResult};
use std::collections::{BTreeSet, HashMap};

#[cfg(test)]
#[path = "skill_catalog_tests.rs"]
mod tests;

/// One row in the rendered catalog. The shape doubles as the JSON
/// envelope (with optional `installed_version`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogEntry {
    /// Marketplace identifier. Today this is the same string as
    /// `name`; preserved as a separate field so the JSON envelope
    /// is stable when `IndexedSkill` grows a distinct id column.
    pub id: String,
    /// Human-readable name (mirrors `id` until separated).
    pub name: String,
    /// Vertical id (currently sourced from `MarketplaceSkill.tenant`).
    pub vertical: String,
    /// Marketplace tags. Empty until the schema grows a tag column.
    pub tags: Vec<String>,
    /// True iff a local `SkillStore::list_active` row's
    /// `frontmatter.name` equals this entry's `name`.
    pub installed: bool,
    /// Marketplace-published version.
    pub version: String,
    /// Local installed version, if any. Omitted from JSON when
    /// `installed` is false.
    pub installed_version: Option<String>,
}

/// Filter knobs collected from the CLI.
#[derive(Debug, Default, Clone, Copy)]
pub struct CatalogFilters<'a> {
    /// Case-insensitive match against marketplace `tenant`.
    pub vertical: Option<&'a str>,
    /// Case-insensitive substring match against description.
    pub tag: Option<&'a str>,
    /// Only emit entries whose `installed` is true.
    pub installed_only: bool,
    /// Only emit entries whose `installed` is false.
    pub not_installed: bool,
}

/// CLI entrypoint. Loads the catalog + installed set, applies
/// filters, and renders either a table or a JSON envelope.
pub async fn run(
    skills: &SkillStore,
    filters: CatalogFilters<'_>,
    json: bool,
) -> ArgoResult<()> {
    let market_rows = fetch_catalog();
    let installed = installed_versions(skills).await?;

    if let Some(vid) = filters.vertical {
        if !known_vertical(&market_rows, vid) {
            return Err(ArgoError::Provider(format!("vertical {vid} not found")));
        }
    }

    let entries = apply_filters(&market_rows, &installed, &filters);

    if market_rows.is_empty() {
        if json {
            println!("[]");
        } else {
            println!("no skills in catalog");
        }
        return Ok(());
    }

    if json {
        render_json(&entries)?;
    } else {
        render_table(&entries);
    }
    Ok(())
}

/// Pull every marketplace entry by issuing an empty-query search at
/// the max limit. `search("", ..)` scores every entry as 1 and skips
/// the score==0 cull, so the result set is the full
/// `MANIFEST_INDEX`. Done this way to avoid duplicating the const
/// index across crates; the public `search` API is the stable
/// surface.
pub fn fetch_catalog() -> Vec<MarketplaceSkill> {
    search(
        "",
        &SearchOptions {
            trust_min: None,
            tenant: None,
            limit: usize::MAX,
        },
    )
}

/// Map of `frontmatter.name -> frontmatter.version` for every
/// active locally-installed skill. The join key is `name` because
/// marketplace entries don't carry the local `SkillId`.
pub async fn installed_versions(skills: &SkillStore) -> ArgoResult<HashMap<String, String>> {
    let active = skills.list_active().await?;
    Ok(active
        .into_iter()
        .map(|s| (s.frontmatter.name, s.frontmatter.version))
        .collect())
}

fn known_vertical(market_rows: &[MarketplaceSkill], vid: &str) -> bool {
    let wanted = vid.to_ascii_lowercase();
    let known: BTreeSet<String> = market_rows
        .iter()
        .map(|r| r.tenant.to_ascii_lowercase())
        .collect();
    known.contains(&wanted)
}

/// Pure filter pipeline — exposed for the sibling unit tests.
pub fn apply_filters(
    market: &[MarketplaceSkill],
    installed: &HashMap<String, String>,
    filters: &CatalogFilters<'_>,
) -> Vec<CatalogEntry> {
    let vertical = filters.vertical.map(str::to_ascii_lowercase);
    let tag = filters.tag.map(str::to_ascii_lowercase);
    market
        .iter()
        .filter_map(|row| {
            if let Some(ref v) = vertical {
                if row.tenant.to_ascii_lowercase() != *v {
                    return None;
                }
            }
            if let Some(ref t) = tag {
                if !row.description.to_ascii_lowercase().contains(t) {
                    return None;
                }
            }
            let installed_version = installed.get(&row.name).cloned();
            let is_installed = installed_version.is_some();
            if filters.installed_only && !is_installed {
                return None;
            }
            if filters.not_installed && is_installed {
                return None;
            }
            Some(CatalogEntry {
                id: row.name.clone(),
                name: row.name.clone(),
                vertical: row.tenant.clone(),
                tags: Vec::new(),
                installed: is_installed,
                version: row.version.clone(),
                installed_version,
            })
        })
        .collect()
}

fn render_table(entries: &[CatalogEntry]) {
    if entries.is_empty() {
        println!("no skills match the supplied filters");
        return;
    }
    println!(" STATUS  ID                                   NAME                         VERTICAL");
    for e in entries {
        let mark = if e.installed { "[x]" } else { "[ ]" };
        let id = &e.id;
        let name = &e.name;
        let vertical = &e.vertical;
        println!(" {mark:<6}  {id:<36} {name:<28} {vertical}");
    }
}

fn render_json(entries: &[CatalogEntry]) -> ArgoResult<()> {
    let payload: Vec<serde_json::Value> = entries.iter().map(entry_to_json).collect();
    let s = serde_json::to_string(&payload)
        .map_err(|e| ArgoError::Provider(format!("serialize catalog: {e}")))?;
    println!("{s}");
    Ok(())
}

fn entry_to_json(e: &CatalogEntry) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    map.insert("id".into(), serde_json::Value::String(e.id.clone()));
    map.insert("name".into(), serde_json::Value::String(e.name.clone()));
    map.insert(
        "vertical".into(),
        serde_json::Value::String(e.vertical.clone()),
    );
    map.insert(
        "tags".into(),
        serde_json::Value::Array(
            e.tags
                .iter()
                .map(|t| serde_json::Value::String(t.clone()))
                .collect(),
        ),
    );
    map.insert("installed".into(), serde_json::Value::Bool(e.installed));
    map.insert(
        "version".into(),
        serde_json::Value::String(e.version.clone()),
    );
    if let Some(iv) = &e.installed_version {
        map.insert(
            "installed_version".into(),
            serde_json::Value::String(iv.clone()),
        );
    }
    serde_json::Value::Object(map)
}
