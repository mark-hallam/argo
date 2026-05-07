//! `argo skill list` — list active or all skills, with filters/format/sort.
use argo_skill::SkillStore;
use argo_types::{ArgoError, ArgoResult, Lock};

#[allow(clippy::too_many_arguments)]
pub(super) async fn list(
    skills: &SkillStore,
    json: bool,
    format: &str,
    sort_by: &str,
    limit: Option<u32>,
    include_superseded: bool,
    name_contains: Option<&str>,
    lock: Option<&str>,
    body_contains: Option<&str>,
    quality_min: Option<u8>,
) -> ArgoResult<()> {
    let lock_filter = match lock {
        None => None,
        Some("none") => Some(Lock::None),
        Some("user-protected") => Some(Lock::UserProtected),
        Some("content-locked") => Some(Lock::ContentLocked),
        Some(other) => {
            return Err(ArgoError::Provider(format!(
                "unknown --lock '{other}' — use none|user-protected|content-locked"
            )));
        }
    };
    let mut active = if include_superseded {
        skills.list_all().await?
    } else {
        skills.list_active().await?
    };
    let quality_map = if quality_min.is_some() {
        let qstore = argo_skill::SkillQualityStore::new(skills.store_arc());
        qstore.ensure_schema().await?;
        qstore.latest_by_skill().await?
    } else {
        std::collections::HashMap::new()
    };
    let needle = name_contains.map(str::to_lowercase);
    let body_needle = body_contains.map(str::to_lowercase);
    active.retain(|s| {
        match needle.as_deref() {
            Some(n) if !s.frontmatter.name.to_lowercase().contains(n) => return false,
            _ => {}
        }
        match body_needle.as_deref() {
            Some(n) if !s.body.to_lowercase().contains(n) => return false,
            _ => {}
        }
        match lock_filter {
            Some(want) if s.frontmatter.lock != want => return false,
            _ => {}
        }
        if let Some(min) = quality_min {
            match quality_map.get(&s.id.0.to_string()) {
                Some(score) if *score >= min => {}
                _ => return false,
            }
        }
        true
    });
    match sort_by {
        "created-at" => active.sort_by_key(|s| std::cmp::Reverse(s.created_at)),
        "name" => active.sort_by(|a, b| a.frontmatter.name.cmp(&b.frontmatter.name)),
        other => {
            return Err(ArgoError::Provider(format!(
                "unknown --sort-by '{other}' — use created-at|name"
            )));
        }
    }
    if let Some(n) = limit {
        active.truncate(n as usize);
    }
    let pick = if json { "json" } else { format };
    match pick {
        "json" => {
            for s in &active {
                let line = serde_json::json!({
                    "id": s.id.0.to_string(),
                    "name": s.frontmatter.name,
                    "version": s.frontmatter.version,
                    "lock": format!("{:?}", s.frontmatter.lock),
                    "hash": hex::encode(s.hash),
                    "created_at": s.created_at.to_rfc3339(),
                });
                let serialized = serde_json::to_string(&line)
                    .map_err(|e| ArgoError::Provider(format!("serialize skill: {e}")))?;
                println!("{serialized}");
            }
            return Ok(());
        }
        "markdown" => {
            println!(
                "# Skills ({} match{})",
                active.len(),
                if active.len() == 1 { "" } else { "es" }
            );
            println!();
            if active.is_empty() {
                return Ok(());
            }
            println!("| name | version | lock | hash |");
            println!("| --- | --- | --- | --- |");
            for s in &active {
                let hash_short: String = hex::encode(s.hash).chars().take(12).collect();
                println!(
                    "| `{}` | `{}` | {:?} | `{hash_short}…` |",
                    s.frontmatter.name, s.frontmatter.version, s.frontmatter.lock,
                );
            }
            return Ok(());
        }
        "text" => {}
        other => {
            return Err(ArgoError::Provider(format!(
                "unknown format '{other}' (expected 'text', 'json', or 'markdown')"
            )));
        }
    }
    if active.is_empty() {
        println!("(no active skills)");
        return Ok(());
    }
    println!("{name:<28} {ver:<8} lock", name = "name", ver = "version");
    for s in active {
        println!(
            "{:<28} {:<8} {:?}",
            s.frontmatter.name, s.frontmatter.version, s.frontmatter.lock
        );
    }
    Ok(())
}
