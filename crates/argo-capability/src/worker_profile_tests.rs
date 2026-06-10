//! Unit tests for the worker-bound least-privilege capability profile
//! and the deterministic irreversibility gate (DEC-025 Genesis fence).

use super::*;
use argo_types::{Capability, SpawnKind};

fn tool(name: &str) -> Capability {
    Capability::ToolDispatch {
        tool_name_glob: name.to_string(),
    }
}

#[test]
fn empty_profile_denies_everything_by_default() {
    let profile = WorkerCapabilityProfile::derive(
        Vec::<String>::new(),
        Vec::<String>::new(),
        Vec::<String>::new(),
        Vec::<String>::new(),
    );
    assert!(!profile.permits_tool("terminal"));
    assert!(!profile.permits_tool("send_message"));
    assert!(!profile.permits_tool("file_write"));
    // Even with an empty allow-set, the to_capability_set carries the
    // minimal always-safe tools but NOTHING side-effecting.
    let set = profile.to_capability_set();
    assert!(set.covers(&tool("clarify")));
    assert!(!set.covers(&tool("terminal")));
    assert!(!set.covers(&tool("send_message")));
}

#[test]
fn derive_grants_declared_actions_and_allowed_tools_only() {
    let profile = WorkerCapabilityProfile::derive(
        ["draft_reply".into(), "summarize_ticket".into()],
        ["web_search".into()],
        Vec::<String>::new(),
        Vec::<String>::new(),
    );
    assert!(profile.permits_tool("draft_reply"));
    assert!(profile.permits_tool("summarize_ticket"));
    assert!(profile.permits_tool("web_search"));
    // Not declared → denied.
    assert!(!profile.permits_tool("terminal"));
    assert!(!profile.permits_tool("file_write"));
}

#[test]
fn deny_wins_over_allow() {
    // A tool listed in both allowed + denied is denied.
    let profile = WorkerCapabilityProfile::derive(
        ["send_email".into()],
        ["send_email".into()],
        ["send_email".into()],
        Vec::<String>::new(),
    );
    assert!(!profile.permits_tool("send_email"));
    let set = profile.to_capability_set();
    assert!(!set.covers(&tool("send_email")));
}

#[test]
fn capability_set_never_grants_star_reach() {
    // The owner role grants ToolDispatch "*"; a derived profile never
    // does, even when the spec declares many tools.
    let profile = WorkerCapabilityProfile::derive(
        ["a".into(), "b".into(), "c".into()],
        ["web_search".into(), "web_extract".into()],
        Vec::<String>::new(),
        Vec::<String>::new(),
    );
    let set = profile.to_capability_set();
    // A wildcard request is NOT covered — the fence is concrete-tool only.
    assert!(!set.covers(&tool("*")));
    assert!(!set.covers(&tool("anything_undeclared")));
    // The concrete declared tools ARE covered.
    assert!(set.covers(&tool("a")));
    assert!(set.covers(&tool("web_search")));
}

#[test]
fn integrations_are_draft_only_not_in_allow_set() {
    // DEC-025 locked #6: MCP/connector servers are recorded as draft,
    // NOT promoted into the live dispatch allow-set at birth.
    let profile = WorkerCapabilityProfile::derive(
        ["draft_reply".into()],
        Vec::<String>::new(),
        Vec::<String>::new(),
        ["gmail".into(), "stripe".into()],
    );
    assert_eq!(profile.draft_integrations(), vec!["gmail", "stripe"]);
    // The integration server names are NOT dispatchable tools.
    assert!(!profile.permits_tool("gmail"));
    assert!(!profile.permits_tool("stripe"));
    let set = profile.to_capability_set();
    assert!(!set.covers(&tool("gmail")));
    assert!(!set.covers(&tool("stripe")));
}

#[test]
fn derive_trims_and_drops_blank_entries() {
    let profile = WorkerCapabilityProfile::derive(
        ["  draft_reply  ".into(), "   ".into(), String::new()],
        ["  web_search ".into()],
        Vec::<String>::new(),
        Vec::<String>::new(),
    );
    assert!(profile.permits_tool("draft_reply"));
    assert!(profile.permits_tool("web_search"));
    assert_eq!(profile.allowed_tools(), vec!["draft_reply", "web_search"]);
}

#[test]
fn irreversible_action_catches_money_data_identity_verbs() {
    assert!(is_irreversible_action("wire_funds"));
    assert!(is_irreversible_action("issue_refund"));
    assert!(is_irreversible_action("process_chargeback"));
    assert!(is_irreversible_action("drop_table"));
    assert!(is_irreversible_action("truncate_log"));
    assert!(is_irreversible_action("wipe_disk"));
    assert!(is_irreversible_action("purge_records"));
    assert!(is_irreversible_action("terminate_employee"));
    assert!(is_irreversible_action("revoke_access"));
    assert!(is_irreversible_action("cancel_subscription"));
    assert!(is_irreversible_action("delete_account"));
}

#[test]
fn irreversible_action_is_case_insensitive() {
    assert!(is_irreversible_action("WIRE_FUNDS"));
    assert!(is_irreversible_action("Drop_Table"));
}

#[test]
fn irreversible_action_does_not_false_positive_on_substrings() {
    // "rewire_config" contains "wire" as a substring but not as a token,
    // and carries no danger stem.
    assert!(!is_irreversible_action("rewire_config"));
    // Benign draft / read / summarize actions are reversible.
    assert!(!is_irreversible_action("draft_summary"));
    assert!(!is_irreversible_action("summarize_ticket"));
    assert!(!is_irreversible_action("read_inbox"));
    assert!(!is_irreversible_action("tag_inventory"));
    // Control: a legit support / customer-success vocabulary stays
    // reversible (no over-flagging of benign work) — the false-positive
    // guard the brief requires for the control role.
    assert!(!is_irreversible_action("triage_ticket"));
    assert!(!is_irreversible_action("draft_response"));
    assert!(!is_irreversible_action("escalate_ticket"));
    assert!(!is_irreversible_action("attach_screenshot"));
    assert!(!is_irreversible_action("display_dashboard"));
    assert!(!is_irreversible_action("reach_out_internally"));
}

#[test]
fn irreversible_action_catches_synonym_families_the_old_token_list_missed() {
    // The live stress-test false-negatives. Each carries a money / destroy /
    // egress / credential family that the original token-exact list let
    // through as AUTONOMOUS + low-risk.
    for name in [
        // money movement
        "process_payment",
        "transfer_funds",
        "remit_to_vendor",
        "disburse_payroll",
        "initiate_payout",
        "withdraw_balance",
        // destructive
        "remove_account_data",
        "erase_records",
        "empty_archived_storage",
        "truncate_audit_log",
        // egress / exfiltration
        "extract_customer_records",
        "export_customer_list",
        "upload_to_partner_system",
        "exfiltrate_pii",
        "download_full_database",
        "forward_to_external",
        // credential entry
        "authenticate_portal",
        "store_credential",
        "submit_password",
    ] {
        assert!(
            is_irreversible_action(name),
            "{name} carries a dangerous family and must be always-human"
        );
    }
}

// ---- DEC-049: bounded worker subagent swarm ----

fn spawn(kind: SpawnKind) -> Capability {
    Capability::Spawn { spawn_kind: kind }
}

#[test]
fn profile_emits_no_spawn_by_default() {
    // The grant is opt-in: a plain profile (even one that allows the
    // delegate_task TOOL) emits NO Spawn capability unless explicitly granted.
    let profile = WorkerCapabilityProfile::derive(
        Vec::<String>::new(),
        ["delegate_task".into()],
        Vec::<String>::new(),
        Vec::<String>::new(),
    );
    let set = profile.to_capability_set();
    assert!(
        !set.covers(&spawn(SpawnKind::Subagent)),
        "delegate_task in allowed_tools alone must NOT grant Spawn — the runtime \
         gates the grant behind ARGO_WORKER_SWARM via with_subagent_spawn"
    );
}

#[test]
fn with_subagent_spawn_grants_only_subagent() {
    let profile = WorkerCapabilityProfile::derive(
        Vec::<String>::new(),
        ["delegate_task".into()],
        Vec::<String>::new(),
        Vec::<String>::new(),
    )
    .with_subagent_spawn(true);
    let set = profile.to_capability_set();
    assert!(set.covers(&spawn(SpawnKind::Subagent)), "granted → Subagent spawn");
    assert!(profile.can_spawn_subagent());
    // The host-escape kinds are NEVER reachable from a worker profile.
    assert!(
        !set.covers(&spawn(SpawnKind::Subprocess)),
        "worker profile must never grant Subprocess (host escape)"
    );
    assert!(
        !set.covers(&spawn(SpawnKind::Container)),
        "worker profile must never grant Container (host escape)"
    );
    assert!(
        !set.covers(&spawn(SpawnKind::BackgroundProcess)),
        "worker profile must never grant BackgroundProcess (strongest host escape)"
    );
}

#[test]
fn with_subagent_spawn_false_revokes() {
    let profile = WorkerCapabilityProfile::derive(
        Vec::<String>::new(),
        ["delegate_task".into()],
        Vec::<String>::new(),
        Vec::<String>::new(),
    )
    .with_subagent_spawn(false);
    assert!(!profile.can_spawn_subagent());
    assert!(!profile
        .to_capability_set()
        .covers(&spawn(SpawnKind::Subagent)));
}

// ---- cu1 (DEC-050 G1): worker-scoped filesystem sandbox grant ----

use std::path::Path;

fn fs_read(glob: &str) -> Capability {
    Capability::FsRead {
        path_glob: glob.to_string(),
    }
}

fn fs_write(glob: &str) -> Capability {
    Capability::FsWrite {
        path_glob: glob.to_string(),
    }
}

#[test]
fn fs_root_with_file_write_grants_scoped_fswrite_only() {
    // Brief acceptance #1: derive(fs_root=/tmp/w) + allowed ["file_write"]
    // → set covers FsWrite{/tmp/w/**} and does NOT cover a path outside the
    // sandbox, Spawn, or a terminal ToolDispatch.
    let profile = WorkerCapabilityProfile::derive(
        Vec::<String>::new(),
        ["file_write".into()],
        Vec::<String>::new(),
        Vec::<String>::new(),
    )
    .with_fs_root(Some(Path::new("/tmp/w")));
    let set = profile.to_capability_set();
    // Covers writes INSIDE the sandbox.
    assert!(set.covers(&fs_write("/tmp/w/report.txt")));
    assert!(set.covers(&fs_write("/tmp/w/sub/dir/a.md")));
    // Never outside the sandbox.
    assert!(!set.covers(&fs_write("/tmp/other/x")));
    assert!(!set.covers(&fs_write("/etc/passwd")));
    // file_write alone does NOT grant read.
    assert!(!set.covers(&fs_read("/tmp/w/report.txt")));
    // No Spawn, no terminal — the sandbox loosens ONLY the fs fence.
    assert!(!set.covers(&spawn(SpawnKind::Subagent)));
    assert!(!set.covers(&spawn(SpawnKind::Subprocess)));
    assert!(!set.covers(&tool("terminal")));
}

#[test]
fn fs_root_with_file_read_grants_scoped_fsread_only() {
    let profile = WorkerCapabilityProfile::derive(
        Vec::<String>::new(),
        ["file_read".into()],
        Vec::<String>::new(),
        Vec::<String>::new(),
    )
    .with_fs_root(Some(Path::new("/tmp/w")));
    let set = profile.to_capability_set();
    assert!(set.covers(&fs_read("/tmp/w/notes.md")));
    assert!(!set.covers(&fs_read("/tmp/other/secret")));
    // Read-only: no write grant.
    assert!(!set.covers(&fs_write("/tmp/w/notes.md")));
}

#[test]
fn fs_root_with_file_patch_grants_fswrite() {
    // file_patch is a mutation tool → FsWrite (not FsRead).
    let profile = WorkerCapabilityProfile::derive(
        Vec::<String>::new(),
        ["file_patch".into()],
        Vec::<String>::new(),
        Vec::<String>::new(),
    )
    .with_fs_root(Some(Path::new("/tmp/w")));
    let set = profile.to_capability_set();
    assert!(set.covers(&fs_write("/tmp/w/app.rs")));
    assert!(!set.covers(&fs_read("/tmp/w/app.rs")));
}

#[test]
fn no_fs_root_emits_no_fs_caps_default_deny_preserved() {
    // Brief acceptance #1 regression pin: fs_root=None → NO Fs caps even when
    // the file tools are in the allow-set. Default-deny is preserved; a worker
    // without a sandbox cannot touch the filesystem at all.
    let profile = WorkerCapabilityProfile::derive(
        Vec::<String>::new(),
        ["file_read".into(), "file_write".into(), "file_patch".into()],
        Vec::<String>::new(),
        Vec::<String>::new(),
    );
    assert_eq!(profile.fs_root(), None);
    let set = profile.to_capability_set();
    assert!(!set.covers(&fs_read("/tmp/w/x")));
    assert!(!set.covers(&fs_write("/tmp/w/x")));
    // Slash-glob form is also not covered.
    assert!(!set.covers(&fs_read("/anything")));
}

#[test]
fn fs_root_without_file_tool_emits_no_fs_caps() {
    // A sandbox set but NO file_* tool in the allow-set → no Fs cap. The grant
    // requires BOTH the sandbox AND the tool.
    let profile = WorkerCapabilityProfile::derive(
        Vec::<String>::new(),
        ["web_search".into()],
        Vec::<String>::new(),
        Vec::<String>::new(),
    )
    .with_fs_root(Some(Path::new("/tmp/w")));
    let set = profile.to_capability_set();
    assert!(!set.covers(&fs_read("/tmp/w/x")));
    assert!(!set.covers(&fs_write("/tmp/w/x")));
}

#[test]
fn fs_root_respects_deny_wins() {
    // file_write in BOTH allowed + denied → deny wins, so no FsWrite cap even
    // with a sandbox.
    let profile = WorkerCapabilityProfile::derive(
        Vec::<String>::new(),
        ["file_read".into(), "file_write".into()],
        ["file_write".into()],
        Vec::<String>::new(),
    )
    .with_fs_root(Some(Path::new("/tmp/w")));
    let set = profile.to_capability_set();
    // Read survives; write is denied.
    assert!(set.covers(&fs_read("/tmp/w/x")));
    assert!(!set.covers(&fs_write("/tmp/w/x")));
}

#[test]
fn with_fs_root_none_clears_the_sandbox() {
    let profile = WorkerCapabilityProfile::derive(
        Vec::<String>::new(),
        ["file_write".into()],
        Vec::<String>::new(),
        Vec::<String>::new(),
    )
    .with_fs_root(Some(Path::new("/tmp/w")))
    .with_fs_root(None);
    assert_eq!(profile.fs_root(), None);
    assert!(!profile
        .to_capability_set()
        .covers(&fs_write("/tmp/w/x")));
}

#[test]
fn fs_root_trailing_slash_normalized() {
    // A root with a trailing slash must not produce a `//**` double-slash glob.
    let profile = WorkerCapabilityProfile::derive(
        Vec::<String>::new(),
        ["file_read".into()],
        Vec::<String>::new(),
        Vec::<String>::new(),
    )
    .with_fs_root(Some(Path::new("/tmp/w/")));
    let set = profile.to_capability_set();
    assert!(set.covers(&fs_read("/tmp/w/notes.md")));
    assert!(!set.covers(&fs_read("/tmp/other/x")));
}

#[test]
fn with_fs_root_rejects_empty_and_traversal_roots() {
    // Defense-in-depth (DEC-050 G1 review): the capability `covers()` glob is
    // lexical, so an empty root (`/**`) or a `..`-containing root would mint a
    // dangerous/escaping glob. `with_fs_root` refuses them at the type boundary
    // → they resolve to None (no Fs grant). (The file tools' confine_path is
    // the real runtime traversal guard; this is belt-and-suspenders.)
    for bad in ["", "   ", "/tmp/w/..", "../../etc", "/tmp/../etc"] {
        let profile = WorkerCapabilityProfile::derive(
            Vec::<String>::new(),
            ["file_write".into(), "file_read".into()],
            Vec::<String>::new(),
            Vec::<String>::new(),
        )
        .with_fs_root(Some(Path::new(bad)));
        let set = profile.to_capability_set();
        assert!(
            !set.covers(&fs_write("/etc/passwd")),
            "root {bad:?} must not mint an escaping FsWrite glob"
        );
        assert!(
            !set.covers(&fs_write("/tmp/w/x")),
            "root {bad:?} must grant NO Fs cap at all"
        );
        assert!(!set.covers(&fs_read("/tmp/w/x")));
    }
    // Sanity: a clean root still grants the scoped cap.
    let ok = WorkerCapabilityProfile::derive(
        Vec::<String>::new(),
        ["file_write".into()],
        Vec::<String>::new(),
        Vec::<String>::new(),
    )
    .with_fs_root(Some(Path::new("/tmp/w")));
    assert!(ok.to_capability_set().covers(&fs_write("/tmp/w/ok.txt")));
}
