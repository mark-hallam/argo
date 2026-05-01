//! `argo skill` — list / view / install / lock / unlock skills.
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
        SkillCmd::List { json, sort_by } => list(&skills, json, &sort_by).await,
        SkillCmd::Count {
            include_superseded,
            json,
        } => count(&skills, include_superseded, json).await,
        SkillCmd::View { name, json } => view(&skills, &name, json).await,
        SkillCmd::Verify { name, all } => {
            if all {
                verify_all(&skills).await
            } else {
                let n = name.unwrap_or_default();
                verify_hash(&skills, &n).await
            }
        }
        SkillCmd::Install { source } => install(&skills, &source).await,
        SkillCmd::Lock { name } => set_lock(&skills, &name, Lock::UserProtected).await,
        SkillCmd::Unlock { name } => set_lock(&skills, &name, Lock::None).await,
        SkillCmd::Refine { name, hint, new_version } => {
            refine(&skills, &name, &hint, new_version.as_deref()).await
        }
        SkillCmd::Create {
            name,
            description,
            body,
            from_file,
            new_version,
        } => create(
            &skills,
            &name,
            description.as_deref(),
            body.as_deref(),
            from_file.as_deref(),
            &new_version,
        )
        .await,
    }
}

async fn create(
    skills: &SkillStore,
    name: &str,
    description: Option<&str>,
    body_arg: Option<&str>,
    from_file: Option<&str>,
    new_version: &str,
) -> ArgoResult<()> {
    if name.is_empty() {
        return Err(ArgoError::Provider("skill create: empty name".into()));
    }
    let body = match (body_arg, from_file) {
        (Some(b), _) => b.to_string(),
        (None, Some(path)) => std::fs::read_to_string(path)
            .map_err(|e| ArgoError::Provider(format!("read {path}: {e}")))?,
        (None, None) => read_stdin_body()?,
    };
    if body.trim().is_empty() {
        return Err(ArgoError::Provider(
            "skill create: empty body — pass --body, --from-file, or pipe text on stdin".into(),
        ));
    }
    let desc = description
        .map(String::from)
        .unwrap_or_else(|| format!("{name} — created via argo skill create"));
    let raw = format!(
        "---\nname: {name}\nversion: \"{new_version}\"\ndescription: {desc}\n\
         lock: none\nprovenance:\n  source: cli-create\n  hash: sha256-{hash}\n---\n{body}",
        hash = body_hash(&body),
    );
    let installed = skills.install(&raw, None).await?;
    println!(
        "argo skill create: installed {name} v{new_version} (id={})",
        installed.id.0
    );
    Ok(())
}

fn read_stdin_body() -> ArgoResult<String> {
    use std::io::Read;
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| ArgoError::Provider(format!("read stdin: {e}")))?;
    Ok(buf)
}

async fn refine(
    skills: &SkillStore,
    name: &str,
    hint: &str,
    version_override: Option<&str>,
) -> ArgoResult<()> {
    let trimmed = hint.trim();
    if trimmed.is_empty() {
        return Err(ArgoError::Provider("skill refine: empty --hint".into()));
    }
    let active = skills
        .find_active_by_name(name)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no active skill named '{name}'")))?;
    let new_version = version_override
        .map(String::from)
        .unwrap_or_else(|| bump_version(&active.frontmatter.version));
    let new_body = format!("{}\n\n## Refinement\n{}\n", active.body.trim_end(), trimmed);
    let raw = format!(
        "---\nname: {name}\nversion: \"{new_version}\"\ndescription: {desc}\n\
         lock: none\nprovenance:\n  source: refined-from-{prev}\n  hash: sha256-{hash}\n---\n{body}",
        desc = active.frontmatter.description,
        prev = active.frontmatter.version,
        hash = body_hash(&new_body),
        body = new_body,
    );
    let new_skill = skills
        .supersede(&raw, None, PrincipalOverride::Force)
        .await?;
    println!(
        "argo skill refine: {name} v{} → v{} (skill_id {})",
        active.frontmatter.version, new_version, new_skill.id.0
    );
    println!("  hint:    {trimmed}");
    println!(
        "  next chat will pick this up via the active-skills system prompt."
    );
    Ok(())
}

/// Bump the trailing numeric segment of a semver-ish version
/// (`"0.1"` → `"0.2"`, `"1.5.3"` → `"1.5.4"`). Falls back to
/// appending `-refined` for non-numeric tails.
fn bump_version(v: &str) -> String {
    let mut parts: Vec<String> = v.split('.').map(String::from).collect();
    if let Some(last) = parts.last_mut() {
        if let Ok(n) = last.parse::<u64>() {
            *last = (n + 1).to_string();
            return parts.join(".");
        }
    }
    format!("{v}-refined")
}

fn body_hash(body: &str) -> String {
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

async fn list(skills: &SkillStore, json: bool, sort_by: &str) -> ArgoResult<()> {
    let mut active = skills.list_active().await?;
    match sort_by {
        "created-at" => active.sort_by_key(|s| std::cmp::Reverse(s.created_at)),
        "name" => active.sort_by(|a, b| a.frontmatter.name.cmp(&b.frontmatter.name)),
        other => {
            return Err(ArgoError::Provider(format!(
                "unknown --sort-by '{other}' — use created-at|name"
            )));
        }
    }
    if json {
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

async fn view(skills: &SkillStore, name: &str, json: bool) -> ArgoResult<()> {
    match skills.find_active_by_name(name).await? {
        None => Err(ArgoError::Provider(format!(
            "no active skill named '{name}'"
        ))),
        Some(s) => {
            if json {
                let row = serde_json::json!({
                    "id": s.id.0.to_string(),
                    "name": s.frontmatter.name,
                    "version": s.frontmatter.version,
                    "description": s.frontmatter.description,
                    "lock": format!("{:?}", s.frontmatter.lock),
                    "hash": hex::encode(s.hash),
                    "body": s.body,
                    "created_at": s.created_at.to_rfc3339(),
                });
                let serialized = serde_json::to_string(&row)
                    .map_err(|e| ArgoError::Provider(format!("serialize skill: {e}")))?;
                println!("{serialized}");
                return Ok(());
            }
            println!("# {} (v{})", s.frontmatter.name, s.frontmatter.version);
            if !s.frontmatter.description.is_empty() {
                println!("{}", s.frontmatter.description);
            }
            println!("\nlock: {:?}", s.frontmatter.lock);
            println!("\n---\n");
            println!("{}", s.body);
            Ok(())
        }
    }
}

async fn verify_all(skills: &SkillStore) -> ArgoResult<()> {
    use sha2::{Digest, Sha256};
    let active = skills.list_active().await?;
    if active.is_empty() {
        println!("(no active skills)");
        return Ok(());
    }
    let mut failures: Vec<String> = Vec::new();
    for s in &active {
        let mut h = Sha256::new();
        h.update(s.body.as_bytes());
        let computed = format!("sha256-{}", hex::encode(h.finalize()));
        if computed == s.frontmatter.provenance.hash {
            println!(
                "  ok       {} v{}",
                s.frontmatter.name, s.frontmatter.version
            );
        } else {
            failures.push(format!(
                "{} v{} stored={} computed={}",
                s.frontmatter.name, s.frontmatter.version, s.frontmatter.provenance.hash, computed
            ));
            println!(
                "  MISMATCH {} v{}",
                s.frontmatter.name, s.frontmatter.version
            );
        }
    }
    if failures.is_empty() {
        println!(
            "all {} skill(s) verified — provenance hashes match",
            active.len()
        );
        Ok(())
    } else {
        Err(ArgoError::Provider(format!(
            "{} of {} skill(s) failed verification:\n  {}",
            failures.len(),
            active.len(),
            failures.join("\n  ")
        )))
    }
}

async fn verify_hash(skills: &SkillStore, name: &str) -> ArgoResult<()> {
    use sha2::{Digest, Sha256};
    let s = skills
        .find_active_by_name(name)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no active skill named '{name}'")))?;
    let mut h = Sha256::new();
    h.update(s.body.as_bytes());
    let computed = format!("sha256-{}", hex::encode(h.finalize()));
    let stored = &s.frontmatter.provenance.hash;
    if &computed == stored {
        println!(
            "skill '{name}' v{} — provenance hash ok ({})",
            s.frontmatter.version, computed
        );
        Ok(())
    } else {
        Err(ArgoError::Provider(format!(
            "skill '{name}' v{} — provenance hash MISMATCH\n  stored:   {stored}\n  computed: {computed}",
            s.frontmatter.version
        )))
    }
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
