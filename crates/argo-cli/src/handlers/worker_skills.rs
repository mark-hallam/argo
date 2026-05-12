//! `argo worker skills <worker_id> --tenant <id>` — list the skills
//! one worker is permitted to surface, derived from
//! [`argo_skill::SkillStore::available_for_worker`] (M6 precursor
//! 2/3, #1318) and the worker's `trust_grade` column (#1273).
//!
//! Operator use case: when a worker isn't seeing an expected skill,
//! `--available-only` narrows the listing to skills that carry a
//! `min_trust_grade` floor so the operator can immediately tell
//! whether trust gating is the cause.
//!
//! Output style mirrors `argo skill list` (text columns by default,
//! `--json` emits one JSON object per skill). Read-only — never
//! writes to the store.
//!
//! This handler is the CLI surface for the mission claim 4
//! propagation loop: a skill installed by Worker A in tenant T is
//! visible to Worker B in T iff B's trust grade clears the skill's
//! floor. The full end-to-end assertion (boot prompt content) lives
//! in `tests/skill_propagation_e2e.rs`.

use crate::home::{db_path, ensure_dir, resolve_home};
use crate::handlers::worker_trust_reader::IdentityWorkerTrustReader;
use argo_identity::{IdentityStore, WorkerId};
use argo_skill::trust::TrustGrade;
use argo_skill::{Skill, SkillStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use std::str::FromStr;
use std::sync::Arc;

/// CLI entry point for `argo worker skills`. Opens the default
/// session store under the operator's `argo` home, then defers to
/// [`gather`] for the SQL + filter pipeline so the unit tests can
/// exercise that pipeline against an in-memory store directly.
pub async fn run(
    worker_id: &str,
    tenant: &str,
    available_only: bool,
    json: bool,
) -> ArgoResult<()> {
    let tenant_id = parse_tenant(tenant)?;
    let worker_uuid = parse_worker_uuid(worker_id)?;
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = Arc::new(SessionStore::open(db_path(&home)).await?);
    store.migrate().await?;
    let identity = IdentityStore::new(&store);
    let reader = IdentityWorkerTrustReader::new(&identity);
    let skills = SkillStore::new(Arc::clone(&store));
    let decorated =
        gather(&skills, &reader, tenant_id, worker_uuid, available_only).await?;
    render(&decorated, json);
    Ok(())
}

/// Resolve the worker's available skills + decorate each row with
/// its `min_trust_grade`. Split out of [`run`] so the unit tests can
/// drive it with an in-memory `SkillStore` + a `FakeTrustReader`
/// instead of touching the operator's filesystem.
pub(crate) async fn gather(
    skills: &SkillStore,
    reader: &dyn argo_skill::WorkerTrustReader,
    tenant_id: TenantId,
    worker_id: uuid::Uuid,
    available_only: bool,
) -> ArgoResult<Vec<(Skill, Option<TrustGrade>)>> {
    let available = skills
        .available_for_worker(reader, tenant_id, worker_id)
        .await?;
    let mut decorated: Vec<(Skill, Option<TrustGrade>)> = Vec::with_capacity(available.len());
    for skill in available {
        let min = skills.get_min_trust_grade(&skill.frontmatter.name).await?;
        if available_only && min.is_none() {
            // `--available-only`: drop unrestricted (NULL-floor) rows
            // so the listing answers "which gated skills passed for
            // this worker?" rather than the full active set.
            continue;
        }
        decorated.push((skill, min));
    }
    Ok(decorated)
}

/// Pure rendering function — split out so the unit tests in
/// `worker_skills_tests.rs` can exercise it without touching
/// `SessionStore`. JSON mode emits one line per skill; the text mode
/// matches `argo skill list`'s column layout.
pub(crate) fn render(skills: &[(Skill, Option<TrustGrade>)], json: bool) {
    if json {
        for (skill, min_trust) in skills {
            let line = serde_json::json!({
                "id": skill.id.0.to_string(),
                "name": skill.frontmatter.name,
                "version": skill.frontmatter.version,
                "min_trust_grade": min_trust.map(format_trust),
            });
            // `serde_json::to_string` cannot fail on a JSON value we
            // just built from owned, well-formed inputs — `unwrap`
            // would be the natural choice but HARD RULE bans it
            // outside `#[cfg(test)]`. Fall back to "{}" so the loop
            // doesn't bail mid-listing if some hypothetical future
            // type breaks the contract.
            let serialized = serde_json::to_string(&line).unwrap_or_else(|_| "{}".to_string());
            println!("{serialized}");
        }
        return;
    }
    if skills.is_empty() {
        println!("(no skills available for this worker)");
        return;
    }
    println!(
        "{name:<28} {ver:<8} min_trust",
        name = "name",
        ver = "version",
    );
    for (skill, min_trust) in skills {
        let trust_label = min_trust.map_or("(none)", format_trust);
        println!(
            "{:<28} {:<8} {}",
            skill.frontmatter.name, skill.frontmatter.version, trust_label,
        );
    }
}

fn format_trust(grade: TrustGrade) -> &'static str {
    match grade {
        TrustGrade::Builtin => "builtin",
        TrustGrade::PublishedVerified => "published_verified",
        TrustGrade::Tenant => "tenant",
        TrustGrade::PublishedCommunity => "published_community",
        TrustGrade::LearnedUnreviewed => "learned_unreviewed",
    }
}

fn parse_tenant(tenant: &str) -> ArgoResult<TenantId> {
    uuid::Uuid::parse_str(tenant)
        .map(TenantId)
        .map_err(|e| ArgoError::Provider(format!("invalid tenant id '{tenant}': {e}")))
}

fn parse_worker_uuid(worker_id: &str) -> ArgoResult<uuid::Uuid> {
    // The CLI accepts either a raw UUID or the `worker-<uuid>` form
    // some operators paste from `argo worker list`. `WorkerId::from_str`
    // handles both; we unpack the inner `Uuid` because
    // `available_for_worker` takes a bare `Uuid` (the trait keeps
    // `argo-skill` free of `argo-identity` deps — see store_available.rs).
    WorkerId::from_str(worker_id)
        .map(|w| w.0)
        .map_err(|e| ArgoError::Provider(format!("invalid worker id '{worker_id}': {e}")))
}

#[cfg(test)]
#[path = "worker_skills_tests.rs"]
mod tests;
