//! `argo skill view <name>` — print one active skill (text or json).
use argo_skill::SkillStore;
use argo_types::{ArgoError, ArgoResult};

pub(super) async fn view(skills: &SkillStore, name: &str, json: bool) -> ArgoResult<()> {
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
