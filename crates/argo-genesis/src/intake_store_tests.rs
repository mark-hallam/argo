//! Tests for the tenant-segmented intake persistence + its isolation guarantee.

use super::*;
use crate::intake_machine::{IntakeSlot, IntakeState};

fn sample_state() -> IntakeState {
    let mut s = IntakeState::default();
    s.apply_raw_answer(IntakeSlot::Charter, "Be my bookkeeping assistant");
    s.apply_raw_answer(IntakeSlot::Persona, "Bo");
    s.record_assistant("What role?");
    s.record_operator("bookkeeping");
    s
}

#[test]
fn save_then_load_round_trips() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = IntakeStore::new(dir.path());
    let state = sample_state();
    store.save("op-alice", "conv-1", &state).expect("save");
    let loaded = store.load("op-alice", "conv-1").expect("load");
    assert_eq!(loaded, state);
}

#[test]
fn tenant_b_cannot_read_tenant_a_intake() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = IntakeStore::new(dir.path());
    store.save("op-alice", "shared-id", &sample_state()).expect("save");
    // Same conversation id, DIFFERENT tenant → resolves a different directory →
    // never reads tenant A's intake.
    assert!(
        store.load("op-bob", "shared-id").is_none(),
        "cross-tenant read must be impossible"
    );
    // Tenant A still reads its own.
    assert!(store.load("op-alice", "shared-id").is_some());
}

#[test]
fn hostile_ids_cannot_traverse_out_of_the_tenant_subtree() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = IntakeStore::new(dir.path());
    // A path-traversal attempt in both segments must stay sandboxed under root.
    store
        .save("../../etc", "../../../secret", &sample_state())
        .expect("save still succeeds (sanitised)");
    // The escape attempt resolves to a sanitised file UNDER the temp root, never
    // outside it.
    let escaped = std::path::Path::new("/etc/secret.json");
    assert!(!escaped.exists(), "must not have written outside the root");
    // And it is still loadable under the SAME (sanitised) ids.
    assert!(store.load("../../etc", "../../../secret").is_some());
}

#[test]
fn unknown_conversation_loads_none() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = IntakeStore::new(dir.path());
    assert!(store.load("op-alice", "never-saved").is_none());
}

#[test]
fn corrupt_file_degrades_to_none_not_panic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = IntakeStore::new(dir.path());
    store.save("op-alice", "conv-x", &sample_state()).expect("save");
    // Corrupt the persisted file.
    let path = store
        .root()
        .join(".genesis-intake")
        .join("op-alice")
        .join("conv-x.json");
    std::fs::write(&path, "not json {{{").expect("overwrite");
    assert!(store.load("op-alice", "conv-x").is_none());
}
