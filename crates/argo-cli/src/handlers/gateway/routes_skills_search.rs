//! `GET /api/v1/skills/search?q=&trust_min=&category=&source=` —
//! server-side ranked search over the caller's REAL installed skills.
//!
//! ## Why this exists (NO THEATER — [`DEC-007`])
//!
//! The SPA `/skills` page previously filtered the catalog with a
//! client-side substring match over the rows it already held. This
//! route moves the ranking server-side and runs it through the SAME
//! engine the marketplace `search` uses: [`argo_skill::rank_skills`]
//! (name > description > tenant/ref weighting, then trust-then-recency
//! tiebreak). The rows it ranks are the tenant's OWN active skill rows
//! (the identical `skill`-table query [`super::routes_skills_list_get`]
//! serves at `/api/v1/skills`) — NOT the baked-in `STATIC_INDEX_DATA`
//! demo table. Wiring the static 6-row demo catalog onto the SPA would
//! replace the operator's real skills with fakes on camera; that is the
//! theater this route deliberately avoids. We reuse the *ranking
//! algorithm*, not a parallel scorer, and we never fabricate rows.
//!
//! ## Facets — derived from REAL columns only
//!
//! The response carries the available facet values computed from the
//! ranked result set, never a hand-authored taxonomy:
//!
//! * `category` — the cohort prefix of the skill `name` (the segment
//!   before the first `.`/`:`/`-`, e.g. `approval.route` → `approval`).
//!   This is exactly the "cohort" label the SPA card already renders.
//! * `source` — the skill's `provenance.source` string.
//! * `trust` — the canonical [`TrustGrade`] discriminant
//!   ([`TrustGrade::from_source`]).
//!
//! ## Tenant scoping
//!
//! The SQL is `WHERE s.tenant_id = ?` (active, non-superseded), so a
//! caller only ever searches/sees their own tenant's skills.
//!
//! [`DEC-007`]: ../../../../../docs/skycot/decisions/DEC-007-no-theater-in-demo.md

use super::routes_skills_list_get::request_tenant_id;
use argo_skill::{rank_skills, MarketplaceSkill, SearchOptions, TrustGrade, DEFAULT_SEARCH_LIMIT};
use argo_store::SessionStore;
use serde_json::{json, Value};
use sqlx::Row;
use std::collections::BTreeMap;
use std::sync::Arc;

const SEARCH_PATH: &str = "/api/v1/skills/search";

/// Dispatch the exact `/api/v1/skills/search` path. Returns `None` for
/// anything else so the `/api/v1/skills/<id>` detail catch-all in
/// [`super::routes_skills_list_get`] still resolves real skill ids.
/// Mounted BEFORE that dispatcher (see [`super::routes_dispatch_get`])
/// so the literal `search` segment is never read as a skill id.
pub(super) async fn dispatch_get(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if !req.starts_with("GET ") {
        return None;
    }
    let after = req.strip_prefix("GET ").unwrap_or(req);
    let request_path = after.split_whitespace().next().unwrap_or("");
    let bare = request_path.split('?').next().unwrap_or(request_path);
    if bare != SEARCH_PATH {
        return None;
    }
    Some(search_response(req, request_path, store).await)
}

/// Parsed query knobs. All optional; absent filters pass everything.
struct SearchQuery {
    q: String,
    trust_min: Option<TrustGrade>,
    /// Cohort prefix filter (case-insensitive exact match on category).
    category: Option<String>,
    /// Provenance-source substring filter (case-insensitive).
    source: Option<String>,
}

async fn search_response(
    req: &str,
    request_path: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let tenant_id = match request_tenant_id(req, store).await {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let query = match parse_query(request_path) {
        Ok(q) => q,
        Err(msg) => return ("400 Bad Request", json!({ "error": msg }).to_string()),
    };

    let rows = match fetch_tenant_skills(store, &tenant_id).await {
        Ok(r) => r,
        Err(e) => {
            return (
                "500 Internal Server Error",
                json!({ "error": format!("list skills: {e}") }).to_string(),
            )
        }
    };

    // Facets are computed over the FULL tenant catalog (the choices the
    // operator could pick), then category/source filters are applied,
    // then the marketplace ranking runs over what survives.
    let facets = facet_values(&rows);
    let filtered = apply_facet_filters(rows, &query);
    let market_rows: Vec<MarketplaceSkill> = filtered.iter().map(|s| s.to_market()).collect();
    let opts = SearchOptions {
        trust_min: query.trust_min,
        tenant: None,
        limit: DEFAULT_SEARCH_LIMIT,
    };
    let ranked = rank_skills(market_rows, &query.q, &opts);

    // Re-join the ranked marketplace rows back to the full skill record
    // (id, layer, risk, tags) by name so the SPA card has everything it
    // already renders. `by_name` keeps the first record per name (active
    // rows are unique per name within a tenant).
    let by_name: BTreeMap<&str, &SkillRecord> =
        filtered.iter().map(|s| (s.name.as_str(), s)).collect();
    let results: Vec<Value> = ranked
        .iter()
        .filter_map(|m| by_name.get(m.name.as_str()).map(|rec| rec.to_card_json()))
        .collect();

    let payload = json!({
        "query": query.q,
        "count": results.len(),
        "results": results,
        "facets": facets,
    });
    ("200 OK", payload.to_string())
}

/// One active skill row, decoded for ranking + the card projection.
struct SkillRecord {
    id: String,
    name: String,
    version: String,
    description: String,
    source: String,
    risk: String,
    tags: Vec<String>,
    created_at_micros: i64,
}

impl SkillRecord {
    /// Project into the marketplace [`MarketplaceSkill`] shape so the
    /// shared ranker can score it. `tenant`/`oci_ref` are not part of
    /// the SPA card and don't affect category/source/trust facets, so
    /// they carry empty/neutral values; the score still weights name +
    /// description exactly as the marketplace search does.
    fn to_market(&self) -> MarketplaceSkill {
        MarketplaceSkill {
            name: self.name.clone(),
            tenant: String::new(),
            description: self.description.clone(),
            version: self.version.clone(),
            trust_grade: TrustGrade::from_source(&self.source),
            last_published_at: micros_to_rfc3339(self.created_at_micros),
            oci_ref: String::new(),
        }
    }

    /// The SPA card envelope — mirrors `routes_skills_list_get`'s
    /// `skill_row_json` so the search grid and the list grid render
    /// identically.
    fn to_card_json(&self) -> Value {
        json!({
            "id": self.id,
            "name": self.name,
            "version": self.version,
            "description": self.description,
            "layer": "workspace",
            "risk": self.risk,
            "tags": self.tags,
            "category": category_of(&self.name),
            "provenance_source": self.source,
            "trust_grade": TrustGrade::from_source(&self.source),
        })
    }
}

/// Tenant-scoped active-skill query — identical predicate to the
/// `/api/v1/skills` list route so search ranks exactly what the list
/// shows.
async fn fetch_tenant_skills(
    store: &Arc<SessionStore>,
    tenant_id: &str,
) -> Result<Vec<SkillRecord>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT s.id, s.name, s.version, s.frontmatter, s.source, s.created_at \
         FROM skill s \
         WHERE s.tenant_id = ? \
           AND s.superseded_by IS NULL \
           AND s.lifecycle_state = 'active' \
         ORDER BY s.name ASC",
    )
    .bind(tenant_id)
    .fetch_all(store.pool())
    .await?;
    Ok(rows.into_iter().map(decode_skill_row).collect())
}

fn decode_skill_row(row: sqlx::sqlite::SqliteRow) -> SkillRecord {
    let id_blob: Vec<u8> = row.try_get("id").unwrap_or_default();
    let id = if id_blob.len() == 16 {
        let mut arr = [0u8; 16];
        arr.copy_from_slice(&id_blob);
        uuid::Uuid::from_bytes(arr).to_string()
    } else {
        String::new()
    };
    let name: String = row.try_get("name").unwrap_or_default();
    let version: String = row.try_get("version").unwrap_or_default();
    let source: String = row.try_get("source").unwrap_or_default();
    let created_at_micros: i64 = row.try_get("created_at").unwrap_or_default();
    let frontmatter: String = row.try_get("frontmatter").unwrap_or_default();
    let (description, risk, tags) = parse_frontmatter_facets(&frontmatter);
    SkillRecord {
        id,
        name,
        version,
        description,
        source,
        risk,
        tags,
        created_at_micros,
    }
}

/// Pull description/risk/tags from the JSON frontmatter blob (best
/// effort — a malformed blob still yields a renderable record). Mirrors
/// the list route's helper of the same name.
fn parse_frontmatter_facets(raw: &str) -> (String, String, Vec<String>) {
    let parsed: Value = serde_json::from_str(raw).unwrap_or(Value::Null);
    let description = parsed
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let risk = parsed
        .get("risk")
        .and_then(Value::as_str)
        .unwrap_or("medium")
        .to_string();
    let tags = parsed
        .get("tags")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    (description, risk, tags)
}

/// Apply the category + source facet filters (the trust filter is
/// applied by the ranker via [`SearchOptions::trust_min`]).
fn apply_facet_filters(rows: Vec<SkillRecord>, query: &SearchQuery) -> Vec<SkillRecord> {
    let category = query.category.as_deref().map(str::to_ascii_lowercase);
    let source = query.source.as_deref().map(str::to_ascii_lowercase);
    rows.into_iter()
        .filter(|r| {
            if let Some(ref cat) = category {
                if category_of(&r.name).to_ascii_lowercase() != *cat {
                    return false;
                }
            }
            if let Some(ref src) = source {
                if !r.source.to_ascii_lowercase().contains(src) {
                    return false;
                }
            }
            true
        })
        .collect()
}

/// Compute the available facet values from the real catalog rows.
/// Categories and sources are sorted-unique; trust grades are the
/// distinct discriminants present, in the canonical (most→least
/// trusted) order. Counts are honest occurrence counts.
fn facet_values(rows: &[SkillRecord]) -> Value {
    let mut categories: BTreeMap<String, usize> = BTreeMap::new();
    let mut sources: BTreeMap<String, usize> = BTreeMap::new();
    let mut trust: BTreeMap<TrustGrade, usize> = BTreeMap::new();
    for r in rows {
        *categories.entry(category_of(&r.name)).or_insert(0) += 1;
        *sources.entry(r.source.clone()).or_insert(0) += 1;
        *trust.entry(TrustGrade::from_source(&r.source)).or_insert(0) += 1;
    }
    let category_facets: Vec<Value> = categories
        .into_iter()
        .map(|(value, count)| json!({ "value": value, "count": count }))
        .collect();
    let source_facets: Vec<Value> = sources
        .into_iter()
        .map(|(value, count)| json!({ "value": value, "count": count }))
        .collect();
    // BTreeMap<TrustGrade,_> iterates in `Ord` order, which for
    // TrustGrade is most-trusted → least-trusted (see its enum doc).
    let trust_facets: Vec<Value> = trust
        .into_iter()
        .map(|(grade, count)| {
            json!({
                "value": trust_label(grade),
                "count": count,
            })
        })
        .collect();
    json!({
        "category": category_facets,
        "source": source_facets,
        "trust": trust_facets,
    })
}

/// The cohort prefix of a skill name — the segment before the first
/// `.`, `:`, or `-`. Falls back to the whole name when there's no
/// separator. This is the same "cohort" the SPA card derives.
fn category_of(name: &str) -> String {
    let end = name
        .find(['.', ':', '-'])
        .unwrap_or(name.len());
    name[..end].to_string()
}

/// Render a [`TrustGrade`] as its serde `snake_case` label (the same
/// string the SPA's `trust_grade` field carries).
fn trust_label(grade: TrustGrade) -> String {
    serde_json::to_value(grade)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| "learned_unreviewed".to_string())
}

/// Parse the query string. `q` defaults to empty (match-all, still
/// ranked). `trust_min` accepts the same labels as the CLI
/// (`published-verified`, `tenant`, …) via
/// [`argo_skill::parse_trust_grade`]; an unknown label is a 400.
fn parse_query(request_path: &str) -> Result<SearchQuery, String> {
    let raw_qs = request_path.split_once('?').map(|(_, qs)| qs).unwrap_or("");
    let q = query_param(raw_qs, "q").unwrap_or_default();
    let category = query_param(raw_qs, "category").filter(|s| !s.is_empty());
    let source = query_param(raw_qs, "source").filter(|s| !s.is_empty());
    let trust_min = match query_param(raw_qs, "trust_min").filter(|s| !s.is_empty()) {
        Some(label) => Some(argo_skill::parse_trust_grade(&label)?),
        None => None,
    };
    Ok(SearchQuery {
        q,
        trust_min,
        category,
        source,
    })
}

/// Minimal percent-decoding query-param reader (handles `+` → space and
/// `%XX` escapes). Self-contained so the route doesn't pull a URL crate
/// for one parameter family.
fn query_param(qs: &str, key: &str) -> Option<String> {
    qs.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        if k != key {
            return None;
        }
        Some(percent_decode(v))
    })
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hi = hex_val(bytes[i + 1]);
                let lo = hex_val(bytes[i + 2]);
                if let (Some(h), Some(l)) = (hi, lo) {
                    out.push((h << 4) | l);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn micros_to_rfc3339(micros: i64) -> String {
    use chrono::{TimeZone, Utc};
    Utc.timestamp_micros(micros)
        .single()
        .unwrap_or(chrono::DateTime::<Utc>::UNIX_EPOCH)
        .to_rfc3339()
}

#[cfg(test)]
#[path = "routes_skills_search_tests.rs"]
mod tests;
