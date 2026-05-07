//! `argo skill` — central dispatch for skill subcommands.
//!
//! Long-tail arms live in per-domain sibling files
//! (`skill_list`, `skill_view`, `skill_verify`, `skill_create`,
//! `skill_refine`, plus the pre-existing
//! `skill_ab` / `skill_quality*` / `skill_review` /
//! `skill_search` / `skill_conflicts`). This file keeps the
//! dispatcher and the small leaf ops (count / install /
//! lock+unlock) along with a shared `body_hash` helper used
//! by `skill_create` and `skill_refine`.
use crate::cli::SkillCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_skill::{PrincipalOverride, SkillStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, Lock};
use std::sync::Arc;

pub async fn run(cmd: SkillCmd) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let db = db_path(&home);
    let store = Arc::new(SessionStore::open(&db).await?);
    store.migrate().await?;
    let skills = SkillStore::new(store);
    match cmd {
        SkillCmd::List {
            json,
            format,
            sort_by,
            limit,
            include_superseded,
            name_contains,
            lock,
            body_contains,
            quality_min,
        } => {
            super::skill_list::list(
                &skills,
                json,
                &format,
                &sort_by,
                limit,
                include_superseded,
                name_contains.as_deref(),
                lock.as_deref(),
                body_contains.as_deref(),
                quality_min,
            )
            .await
        }
        SkillCmd::Count {
            include_superseded,
            json,
        } => count(&skills, include_superseded, json).await,
        SkillCmd::View { name, json } => super::skill_view::view(&skills, &name, json).await,
        SkillCmd::Verify { name, all } => {
            if all {
                super::skill_verify::verify_all(&skills).await
            } else {
                let n = name.unwrap_or_default();
                super::skill_verify::verify_hash(&skills, &n).await
            }
        }
        SkillCmd::Install { source } => install(&skills, &source).await,
        SkillCmd::DistillNow { worker, json } => {
            super::skill_distill_now::run_cli(&worker, json).await
        }
        SkillCmd::Export { name, out, json } => {
            super::skill_export::export_one(&skills, &name, &out, json).await
        }
        SkillCmd::ExportAll {
            out,
            vertical,
            json,
        } => super::skill_export_all::run(&skills, &out, vertical.as_deref(), json).await,
        SkillCmd::Import {
            path,
            name,
            workspace,
            dry_run,
            json,
        } => {
            super::skill_import::import_artifact(
                &skills,
                &path,
                name.as_deref(),
                workspace.as_deref(),
                dry_run,
                json,
            )
            .await
        }
        SkillCmd::Lock { name } => set_lock(&skills, &name, Lock::UserProtected).await,
        SkillCmd::Unlock { name } => set_lock(&skills, &name, Lock::None).await,
        SkillCmd::Refine {
            name,
            hint,
            new_version,
        } => super::skill_refine::refine(&skills, &name, &hint, new_version.as_deref()).await,
        SkillCmd::Review { cmd } => super::skill_review::run(&skills, cmd).await,
        SkillCmd::Search(args) => super::skill_search::run(
            &args.query,
            args.trust_min.as_deref(),
            args.tenant.as_deref(),
            args.limit,
            args.json,
            args.vertical.as_deref(),
            args.include_used_by_sessions,
        )
        .await,
        SkillCmd::Publish(args) => super::skill_publish::run(&skills, args).await,
        SkillCmd::Unpublish(args) => super::skill_unpublish::run(args),
        SkillCmd::Tags(args) => super::skill_tags::run(args),
        SkillCmd::Runs {
            name,
            tenant,
            worker,
            since,
            limit,
            include_failed,
            json,
        } => {
            super::skill_runs::run(
                name.as_deref(),
                tenant.as_deref(),
                worker.as_deref(),
                since.as_deref(),
                limit,
                include_failed,
                json,
            )
            .await
        }
        SkillCmd::Conflicts { cmd } => super::skill_conflicts::run(&skills, cmd).await,
        SkillCmd::Create {
            name,
            description,
            body,
            from_file,
            new_version,
        } => {
            super::skill_create::create(
                &skills,
                &name,
                description.as_deref(),
                body.as_deref(),
                from_file.as_deref(),
                &new_version,
            )
            .await
        }
        SkillCmd::Versions(args) => super::skill_ab::versions(args).await,
        SkillCmd::AbTest(args) => super::skill_ab::ab_test(args).await,
        SkillCmd::AbStatus(args) => super::skill_ab::ab_status(args).await,
        SkillCmd::AbPromote(args) => super::skill_ab::ab_promote(args).await,
        SkillCmd::Quality {
            name,
            json,
            format,
            limit,
        } => super::skill_quality::run(&name, json, &format, limit).await,
        SkillCmd::Deps {
            name,
            json,
            reverse,
            max_depth,
        } => super::skill_deps::run(&name, json, reverse, max_depth).await,
        SkillCmd::QualityTrend { name, days, json } => {
            super::skill_quality_trend::run(&name, days, json).await
        }
        SkillCmd::QualityList {
            threshold,
            worst,
            json,
        } => super::skill_quality_list::run(threshold, worst, json).await,
        SkillCmd::QualityRecompute {
            name,
            all,
            success_rate,
            avg_latency_ms,
            retry_count,
            p99_latency_ms,
            json,
        } => {
            super::skill_quality_recompute::run(
                name.as_deref(),
                all,
                success_rate,
                avg_latency_ms,
                retry_count,
                p99_latency_ms,
                json,
            )
            .await
        }
    }
}

/// SHA-256 of a skill body, hex-encoded (no `sha256-` prefix).
/// Shared between `skill_create` and `skill_refine`.
pub(super) fn body_hash(body: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(body.as_bytes());
    hex::encode(h.finalize())
}

/// `argo skill count [--include-superseded] [--json]` — print
/// just the count of active skills (or all versions with
/// --include-superseded). Default: one int per line.
async fn count(skills: &SkillStore, include_superseded: bool, json: bool) -> ArgoResult<()> {
    let n = if include_superseded {
        skills.list_all().await?.len()
    } else {
        skills.list_active().await?.len()
    };
    if json {
        let payload = serde_json::json!({"count": n});
        println!("{}", serde_json::to_string(&payload).unwrap_or_default());
    } else {
        println!("{n}");
    }
    Ok(())
}

async fn install(skills: &SkillStore, source: &str) -> ArgoResult<()> {
    let body = std::fs::read_to_string(source)
        .map_err(|e| ArgoError::Provider(format!("read {source}: {e}")))?;
    let installed = skills.install(&body, None).await?;
    println!(
        "installed {} v{} (id={})",
        installed.frontmatter.name, installed.frontmatter.version, installed.id.0
    );
    Ok(())
}

async fn set_lock(skills: &SkillStore, name: &str, lock: Lock) -> ArgoResult<()> {
    let s = skills
        .find_active_by_name(name)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no active skill named '{name}'")))?;
    skills
        .set_lock(s.id, lock, PrincipalOverride::Force)
        .await?;
    println!("skill '{name}' lock = {lock:?}");
    Ok(())
}
