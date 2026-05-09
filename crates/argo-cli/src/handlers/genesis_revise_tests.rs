use super::*;
use argo_genesis::{plan, write_vertical};
use std::path::Path;

fn scaffold_vertical(parent: &Path) -> (String, std::path::PathBuf) {
    let p = plan(
        "be my marketing intern, run weekly campaigns to cold leads",
        "maya",
    )
    .expect("plan");
    let out = parent.join(&p.vertical_id);
    write_vertical(&p, &out).expect("write");
    (p.vertical_id, out)
}

fn args_for(parent: &Path, id: &str, target: &str, dry_run: bool) -> GenesisReviseArgs {
    GenesisReviseArgs {
        vertical: id.to_string(),
        feedback: "be more terse and skip social-media actions".into(),
        target: target.into(),
        dry_run,
        json: true,
        verticals_dir: Some(parent.to_path_buf()),
    }
}

#[test]
fn revise_target_prompt_updates_only_system_prompt() {
    let dir = tempfile::tempdir().unwrap();
    let (id, vertical_dir) = scaffold_vertical(dir.path());
    let prompt_path = vertical_dir.join("prompts/system.md");
    let manifest_path = vertical_dir.join("vertical.toml");
    let eval_path = vertical_dir.join("eval-seed.toml");
    let prompt_before = std::fs::read_to_string(&prompt_path).unwrap();
    let manifest_before = std::fs::read_to_string(&manifest_path).unwrap();
    let eval_before = std::fs::read_to_string(&eval_path).unwrap();

    handle_revise(args_for(dir.path(), &id, "prompt", false)).expect("revise prompt");

    let prompt_after = std::fs::read_to_string(&prompt_path).unwrap();
    assert_ne!(prompt_after, prompt_before, "prompt should change");
    assert_eq!(
        std::fs::read_to_string(&manifest_path).unwrap(),
        manifest_before,
        "vertical.toml must not change for --target prompt"
    );
    assert_eq!(
        std::fs::read_to_string(&eval_path).unwrap(),
        eval_before,
        "eval-seed.toml must not change for --target prompt"
    );
}

#[test]
fn revise_target_all_updates_all_three_artefacts() {
    let dir = tempfile::tempdir().unwrap();
    let (id, vertical_dir) = scaffold_vertical(dir.path());
    let prompt_path = vertical_dir.join("prompts/system.md");
    let manifest_path = vertical_dir.join("vertical.toml");
    let eval_path = vertical_dir.join("eval-seed.toml");
    let prompt_before = std::fs::read_to_string(&prompt_path).unwrap();
    let manifest_before = std::fs::read_to_string(&manifest_path).unwrap();
    let eval_before = std::fs::read_to_string(&eval_path).unwrap();

    handle_revise(args_for(dir.path(), &id, "all", false)).expect("revise all");

    assert_ne!(
        std::fs::read_to_string(&prompt_path).unwrap(),
        prompt_before,
        "prompt should change"
    );
    let manifest_after = std::fs::read_to_string(&manifest_path).unwrap();
    let eval_after = std::fs::read_to_string(&eval_path).unwrap();
    assert!(
        manifest_after != manifest_before || eval_after != eval_before,
        "at least one of manifest/eval should change for --target all"
    );
}

#[test]
fn revise_dry_run_does_not_write_to_disk() {
    let dir = tempfile::tempdir().unwrap();
    let (id, vertical_dir) = scaffold_vertical(dir.path());
    let prompt_path = vertical_dir.join("prompts/system.md");
    let manifest_path = vertical_dir.join("vertical.toml");
    let eval_path = vertical_dir.join("eval-seed.toml");
    let prompt_before = std::fs::read_to_string(&prompt_path).unwrap();
    let manifest_before = std::fs::read_to_string(&manifest_path).unwrap();
    let eval_before = std::fs::read_to_string(&eval_path).unwrap();

    handle_revise(args_for(dir.path(), &id, "all", true)).expect("dry-run revise all");

    assert_eq!(
        std::fs::read_to_string(&prompt_path).unwrap(),
        prompt_before,
        "dry-run must not modify the prompt"
    );
    assert_eq!(
        std::fs::read_to_string(&manifest_path).unwrap(),
        manifest_before,
        "dry-run must not modify the manifest"
    );
    assert_eq!(
        std::fs::read_to_string(&eval_path).unwrap(),
        eval_before,
        "dry-run must not modify the eval seed"
    );
}

#[test]
fn revise_missing_vertical_returns_provider_error() {
    let dir = tempfile::tempdir().unwrap();
    let err = handle_revise(args_for(dir.path(), "no-such-vertical", "all", true)).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("not found"),
        "missing vertical should surface a not-found error; got: {msg}"
    );
}

#[test]
fn revise_preserves_locked_keep_sections_in_prompt() {
    let dir = tempfile::tempdir().unwrap();
    let (id, vertical_dir) = scaffold_vertical(dir.path());
    let prompt_path = vertical_dir.join("prompts/system.md");
    let mut original = std::fs::read_to_string(&prompt_path).unwrap();
    let locked = "[KEEP]Operator-approved tone: warm, signed `— maya`.[/KEEP]";
    original.push_str("\n\n## Operator overrides\n\n");
    original.push_str(locked);
    original.push('\n');
    std::fs::write(&prompt_path, &original).unwrap();

    handle_revise(args_for(dir.path(), &id, "prompt", false)).expect("revise");

    let after = std::fs::read_to_string(&prompt_path).unwrap();
    assert!(
        after.contains(locked),
        "[KEEP]…[/KEEP] block must survive a revise pass; got:\n{after}"
    );
}

#[test]
fn revise_target_string_parses_known_values_and_rejects_garbage() {
    assert!(matches!("prompt".parse::<ReviseTarget>(), Ok(ReviseTarget::Prompt)));
    assert!(matches!("ACTIONS".parse::<ReviseTarget>(), Ok(ReviseTarget::Actions)));
    assert!(matches!("eval".parse::<ReviseTarget>(), Ok(ReviseTarget::Eval)));
    assert!(matches!("all".parse::<ReviseTarget>(), Ok(ReviseTarget::All)));
    assert!("nonsense".parse::<ReviseTarget>().is_err());
}

#[test]
fn revise_rejects_empty_feedback() {
    let dir = tempfile::tempdir().unwrap();
    let (id, _) = scaffold_vertical(dir.path());
    let mut args = args_for(dir.path(), &id, "prompt", true);
    args.feedback = "   ".into();
    let err = handle_revise(args).unwrap_err();
    assert!(err.to_string().contains("feedback"), "got: {err}");
}
