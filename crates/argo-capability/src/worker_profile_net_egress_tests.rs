//! cu1d (DEC-050 G1 follow-up) NetEgress tests for the worker capability
//! fence, split out of `worker_profile_tests.rs` to keep both files under
//! the 500-LOC ceiling. Verifies the web-research-tool opt-in emits
//! `NetEgress{**}` and that default-deny + deny-wins are preserved.

use super::*;
use argo_types::{Capability, SpawnKind};
use std::path::Path;

fn net_egress_any() -> Capability {
    Capability::NetEgress {
        host_glob: "**".to_string(),
        port: None,
    }
}

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

fn spawn(kind: SpawnKind) -> Capability {
    Capability::Spawn { spawn_kind: kind }
}

#[test]
fn web_search_in_allow_set_grants_net_egress() {
    // Manifest opt-in to a web research tool → the dispatch-required
    // NetEgress is emitted, so `web_search` doesn't dead-end at rbac.
    let profile = WorkerCapabilityProfile::derive(
        Vec::<String>::new(),
        ["web_search".into()],
        Vec::<String>::new(),
        Vec::<String>::new(),
    );
    let set = profile.to_capability_set();
    assert!(set.covers(&net_egress_any()));
    // Egress alone loosens NOTHING else: no fs, no spawn, no shell.
    assert!(!set.covers(&fs_read("/tmp/w/x")));
    assert!(!set.covers(&fs_write("/tmp/w/x")));
    assert!(!set.covers(&spawn(SpawnKind::Subagent)));
    assert!(!set.covers(&spawn(SpawnKind::Subprocess)));
}

#[test]
fn web_extract_in_allow_set_grants_net_egress() {
    let profile = WorkerCapabilityProfile::derive(
        Vec::<String>::new(),
        ["web_extract".into()],
        Vec::<String>::new(),
        Vec::<String>::new(),
    );
    assert!(profile.to_capability_set().covers(&net_egress_any()));
}

#[test]
fn no_web_tools_means_no_net_egress() {
    // Regression pin: the default-deny posture is unchanged — a worker
    // with file tools + declared actions but no web tools has NO egress.
    let profile = WorkerCapabilityProfile::derive(
        ["send_payment_reminder".into()],
        ["file_read".into(), "file_write".into()],
        Vec::<String>::new(),
        Vec::<String>::new(),
    )
    .with_fs_root(Some(Path::new("/tmp/w")));
    assert!(!profile.to_capability_set().covers(&net_egress_any()));
}

#[test]
fn denied_web_search_wins_over_allowed_no_net_egress() {
    // Deny-wins: an explicit deny on the web tool suppresses the grant
    // even when the allow-set lists it.
    let profile = WorkerCapabilityProfile::derive(
        Vec::<String>::new(),
        ["web_search".into(), "web_extract".into()],
        ["web_search".into(), "web_extract".into()],
        Vec::<String>::new(),
    );
    assert!(!profile.to_capability_set().covers(&net_egress_any()));
}
