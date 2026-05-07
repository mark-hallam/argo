//! `argo skill create` — install a new skill from CLI input (--body / --from-file / stdin).
use super::skill::body_hash;
use argo_skill::SkillStore;
use argo_types::{ArgoError, ArgoResult};

pub(super) async fn create(
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
