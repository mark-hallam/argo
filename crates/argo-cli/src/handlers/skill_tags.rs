//! `argo skill tags` — list published marketplace tags.

use crate::cli::SkillTagsCmd;
use argo_skill::{list_mock_skill_tags, mock_registry_root, SkillTag};
use argo_types::ArgoResult;

pub fn run(args: SkillTagsCmd) -> ArgoResult<()> {
    let root = mock_registry_root(&args.registry)?;
    let rows = list_mock_skill_tags(&root, &args.name, args.limit)?;
    if args.json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "ok": true,
                "command": "tags",
                "registry": args.registry,
                "name": args.name,
                "tags": rows,
            }))
            .unwrap_or_default()
        );
    } else {
        render(&rows);
    }
    Ok(())
}

fn render(rows: &[SkillTag]) {
    if rows.is_empty() {
        println!("(no published tags)");
        return;
    }
    println!(
        "{:<28} {:<12} {:<20} digest",
        "skill", "tag", "published_at"
    );
    for row in rows {
        println!(
            "{:<28} {:<12} {:<20} {}",
            row.name, row.tag, row.published_at, row.digest
        );
    }
}
