//! `argo skill unpublish` — remove a marketplace tag pointer.

use crate::cli::SkillUnpublishCmd;
use argo_skill::{mock_registry_root, unpublish_mock_skill_tag};
use argo_types::ArgoResult;

pub fn run(args: SkillUnpublishCmd) -> ArgoResult<()> {
    let root = mock_registry_root(&args.registry)?;
    let record = unpublish_mock_skill_tag(&root, &args.reference)?;
    if args.json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "ok": true,
                "command": "unpublish",
                "registry": args.registry,
                "record": record,
            }))
            .unwrap_or_default()
        );
    } else if record.removed {
        println!(
            "unpublished {}:{} (blob preserved)",
            record.name, record.tag
        );
    } else {
        println!("tag not found: {}:{}", record.name, record.tag);
    }
    Ok(())
}
