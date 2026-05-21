//! `GET /skills?risk=...` + `GET /skills/<name>` + `GET /skills/<name>/usage`
//! — layered skill lookup routes for the worker runtime, review UI,
//! and the skill-discovery marketplace surface (mission-skill-discovery).
use argo_identity::IdentityStore;
use argo_skill::{LayeredLookup, LayeredSkill, SkillStore, TrustGrade};
use argo_store::SessionStore;
use chrono::Datelike;
use serde_json::Value;
use std::sync::Arc;

pub(super) async fn dispatch_route(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if req.starts_with("GET /skills/learned-this-month ")
        || req.starts_with("GET /skills/learned-this-month?")
    {
        return Some(skills_learned_this_month_response(store).await);
    }

    if req.starts_with("GET /skills ") || req.starts_with("GET /skills?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let risk = super::routes_query::parse_query_kv(tail, "risk");
        return Some(skills_list_response(store, risk.as_deref()).await);
    }

    let rest = req.strip_prefix("GET /skills/")?;
    let (name_with_query, _) = rest.split_once(' ')?;
    let name_no_query = name_with_query
        .split_once('?')
        .map(|(name, _)| name)
        .unwrap_or(name_with_query);

    // /skills/<name>/usage — provenance + workers-using + adoption count.
    if let Some(name) = name_no_query.strip_suffix("/usage") {
        if name.is_empty() || name.contains('/') {
            return None;
        }
        return Some(skill_usage_response(store, name).await);
    }

    if name_no_query == "count" || name_no_query.contains('/') {
        return None;
    }
    Some(skill_show_response(store, name_no_query).await)
}

async fn skills_list_response(
    store: Option<&Arc<SessionStore>>,
    raw_risk: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let skills = SkillStore::new(Arc::clone(store));
    let lookup = LayeredLookup::workspace_only(&skills);
    list_response_for_lookup(&lookup, raw_risk).await
}

async fn skill_show_response(
    store: Option<&Arc<SessionStore>>,
    name: &str,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let skills = SkillStore::new(Arc::clone(store));
    let lookup = LayeredLookup::workspace_only(&skills);
    show_response_for_lookup(&lookup, name).await
}

/// `GET /skills/<name>/usage` — workers in the current tenant using the
/// skill (with `learned_at`), the global adoption count (integer only —
/// no cross-tenant identity leaks), an anonymized "Authored by ..."
/// label, and the canonical `trust_grade`. Backs the
/// mission-skill-discovery marketplace surface.
async fn skill_usage_response(
    store: Option<&Arc<SessionStore>>,
    name: &str,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let skills = SkillStore::new(Arc::clone(store));
    let active = match skills.find_active_by_name(name).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return (
                "404 Not Found",
                json_error(&format!("no active skill named '{name}'")),
            );
        }
        Err(e) => {
            return (
                "500 Internal Server Error",
                json_error(&format!("skill lookup: {e}")),
            );
        }
    };

    let tenant = match resolve_workspace_tenant(store).await {
        Ok(t) => t,
        Err(e) => {
            return (
                "500 Internal Server Error",
                json_error(&format!("resolve tenant: {e}")),
            );
        }
    };

    let identity = IdentityStore::new(store);
    let skill_id_str = active.id.0.to_string();

    let workers_in_tenant = match identity
        .workers_with_skill_learned_at(tenant, &skill_id_str)
        .await
    {
        Ok(v) => v,
        Err(e) => {
            return (
                "500 Internal Server Error",
                json_error(&format!("workers_with_skill: {e}")),
            );
        }
    };

    let adoption_count = match identity.count_workers_with_skill_global(&skill_id_str).await {
        Ok(c) => c,
        Err(e) => {
            return (
                "500 Internal Server Error",
                json_error(&format!("adoption count: {e}")),
            );
        }
    };

    let workers_json: Vec<Value> = workers_in_tenant
        .iter()
        .map(|(worker, learned_at)| {
            serde_json::json!({
                "worker_id": worker.id.to_string(),
                "name": worker.persona.name,
                "learned_at": learned_at,
            })
        })
        .collect();

    let author_label = author_label_for_source(&active.frontmatter.provenance.source);
    let payload = serde_json::json!({
        "skill_id": skill_id_str,
        "skill_name": active.frontmatter.name,
        "trust_grade": TrustGrade::from_source(&active.frontmatter.provenance.source),
        "author_label": author_label,
        "workers_using": workers_json,
        "workers_using_count": workers_in_tenant.len(),
        "adoption_count": adoption_count,
    });
    (
        "200 OK",
        serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()),
    )
}

/// `GET /skills/learned-this-month` — skill ids the workers in the
/// current tenant have learned since the first-of-month (UTC). Backs
/// the marketplace's "Skills my workers learned this month" filter
/// chip; the response is `{ since, count, skill_ids }` so the web
/// client can intersect with the existing `/skills` registry list.
async fn skills_learned_this_month_response(
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let tenant = match resolve_workspace_tenant(store).await {
        Ok(t) => t,
        Err(e) => {
            return (
                "500 Internal Server Error",
                json_error(&format!("resolve tenant: {e}")),
            );
        }
    };
    let since = month_start_iso(chrono::Utc::now());
    let identity = IdentityStore::new(store);
    match identity.skill_ids_learned_since(tenant, &since).await {
        Ok(ids) => {
            let payload = serde_json::json!({
                "since": since,
                "count": ids.len(),
                "skill_ids": ids,
            });
            (
                "200 OK",
                serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            json_error(&format!("learned_since: {e}")),
        ),
    }
}

/// First-of-month (UTC) for `now` as an RFC3339 string. Pure helper so
/// the test below can pin the boundary without touching wall clock.
fn month_start_iso(now: chrono::DateTime<chrono::Utc>) -> String {
    let naive = now.date_naive();
    let first_date = chrono::NaiveDate::from_ymd_opt(naive.year(), naive.month(), 1).unwrap_or(naive);
    // `and_hms_opt(0,0,0)` is infallible for any valid `NaiveDate`, so an
    // `unwrap` here would clippy-fire under `deny(unwrap_used)`. Fall
    // back to `naive_date.and_time(MIN)` instead to keep the lint happy.
    let first = first_date.and_time(chrono::NaiveTime::MIN);
    chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(first, chrono::Utc).to_rfc3339()
}

/// Resolve the workspace-bound tenant for read-only usage queries.
/// Mirrors `routes_cohorts::resolve_workspace_tenant`; kept local so the
/// crate stays under HARD RULE 6's 500-LOC per-file ceiling.
async fn resolve_workspace_tenant(
    store: &SessionStore,
) -> argo_types::ArgoResult<argo_types::TenantId> {
    let cwd = std::env::current_dir()
        .map_err(|e| argo_types::ArgoError::Io(format!("current_dir: {e}")))?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _) = store.ensure_workspace(name, &root).await?;
    Ok(tenant)
}

/// Map a skill's `provenance.source` to the operator-facing
/// authorship label. Plain-language per the brief: "Authored by
/// SKYCOT" for shipped-in / marketplace-published skills, "Authored
/// by your org" otherwise. Anonymized — no per-tenant identity is
/// surfaced.
fn author_label_for_source(source: &str) -> &'static str {
    // Builtins, vertical-packs, and verified-marketplace skills are
    // SKYCOT-authored. Community / learned / workspace skills came
    // from inside the operator's own org.
    if source == "builtin"
        || source == "verticals"
        || source.starts_with("published:verified:market")
        || source.starts_with("published:community:market")
    {
        "Authored by SKYCOT"
    } else {
        "Authored by your org"
    }
}

async fn list_response_for_lookup(
    lookup: &LayeredLookup<'_>,
    raw_risk: Option<&str>,
) -> (&'static str, String) {
    let (risk, risk_defaulted) = match parse_risk(raw_risk) {
        Ok(r) => r,
        Err(msg) => return ("400 Bad Request", json_error(&msg)),
    };
    match lookup.list_for_action(risk).await {
        Ok(rows) => {
            let skills: Vec<serde_json::Value> = rows.iter().map(layered_skill_json).collect();
            let payload = serde_json::json!({
                "risk": risk,
                "risk_defaulted": risk_defaulted,
                "count": skills.len(),
                "skills": skills,
            });
            (
                "200 OK",
                serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            json_error(&format!("layered skills list: {e}")),
        ),
    }
}

async fn show_response_for_lookup(
    lookup: &LayeredLookup<'_>,
    name: &str,
) -> (&'static str, String) {
    match lookup.find(name).await {
        Ok(Some(row)) => (
            "200 OK",
            serde_json::to_string(&layered_skill_json(&row)).unwrap_or_else(|_| "{}".to_string()),
        ),
        Ok(None) => (
            "404 Not Found",
            json_error(&format!(
                "no active skill named '{name}' in workspace or library"
            )),
        ),
        Err(e) => (
            "500 Internal Server Error",
            json_error(&format!("layered skill show: {e}")),
        ),
    }
}

fn parse_risk(raw: Option<&str>) -> Result<(&'static str, bool), String> {
    match raw {
        None => Ok(("medium", true)),
        Some("low") => Ok(("low", false)),
        Some("medium") => Ok(("medium", false)),
        Some("high") => Ok(("high", false)),
        Some(other) => Err(format!("invalid risk '{other}'; use low|medium|high")),
    }
}

fn layered_skill_json(row: &LayeredSkill) -> serde_json::Value {
    let skill = &row.skill;
    serde_json::json!({
        "id": skill.id.0.to_string(),
        "name": skill.frontmatter.name,
        "version": skill.frontmatter.version,
        "description": skill.frontmatter.description,
        "layer": row.layer,
        "provenance_source": skill.frontmatter.provenance.source,
        "trust_grade": TrustGrade::from_source(&skill.frontmatter.provenance.source),
        "lock": format!("{:?}", skill.frontmatter.lock),
        "hash": hex::encode(skill.hash),
        "body": skill.body,
        "created_at": skill.created_at.to_rfc3339(),
    })
}

fn json_error(msg: &str) -> String {
    serde_json::json!({ "error": msg }).to_string()
}

#[cfg(test)]
#[path = "routes_skills_tests.rs"]
mod tests;
