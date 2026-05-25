//! E2E spec for `POST /api/v1/genesis` (mission-genesis-gateway-
//! route-streaming-api).
//!
//! `argo-cli` is a bin crate with no exported library, so this file
//! cannot reach the SSE handler directly. Instead it exercises the
//! contract the route stands up — using the same `argo-genesis` +
//! `argo-vertical` public APIs the handler chains together — and
//! pins the brief's two load-bearing claims:
//!
//! 1. Generated vertical round-trips cleanly through
//!    `argo_vertical::load_dir`.
//! 2. Tenant A's generated vertical is NOT visible to a load_dir
//!    scoped to tenant B's filesystem subtree (the cross-tenant
//!    isolation property `GET /api/v1/verticals` relies on).
//!
//! Full request-path coverage (env-var gate, SSE wire format,
//! 4-phase progress + final `vertical_ready`) lives in
//! `crates/argo-cli/src/handlers/gateway/routes_genesis_stream_tests.rs`.

use argo_genesis::{name_to_persona_slug, plan, write_vertical};
use argo_vertical::load_dir;
use std::path::PathBuf;
use tempfile::TempDir;

const ROLE_DESC_A: &str =
    "I need a customer success lead who triages tier-1 tickets and escalates refunds over $500.";
const ROLE_DESC_B: &str =
    "I need a marketing intern who runs weekly newsletter campaigns to cold B2B SaaS leads.";

fn genesis_dir(root: &std::path::Path, tenant_id: &str) -> PathBuf {
    // Mirrors `routes_genesis_stream::resolve_vertical_dir` minus
    // the per-vertical leaf so tests can drop multiple generated
    // verticals under the same tenant.
    root.join(".genesis").join(tenant_id)
}

#[test]
fn generated_vertical_round_trips_through_argo_vertical_load_dir() {
    let tmp = TempDir::new().unwrap();
    let tenant_dir = genesis_dir(tmp.path(), "tenant-a");
    std::fs::create_dir_all(&tenant_dir).unwrap();
    let persona = name_to_persona_slug("Casey");
    let plan_a = plan(ROLE_DESC_A, &persona).expect("genesis plan");
    let vertical_path = tenant_dir.join(&plan_a.vertical_id);
    write_vertical(&plan_a, &vertical_path).expect("write vertical");

    let (registry, errors) = load_dir(&tenant_dir).expect("load_dir tenant subtree");
    assert!(
        errors.is_empty(),
        "expected zero load errors; got: {errors:?}"
    );
    let loaded = registry
        .get(&plan_a.vertical_id)
        .expect("vertical present in registry");
    assert_eq!(loaded.manifest.vertical.id, plan_a.vertical_id);
    assert!(!loaded.manifest.vertical.name.is_empty());
}

#[test]
fn cross_tenant_load_dir_does_not_leak_other_tenants_verticals() {
    let tmp = TempDir::new().unwrap();
    let tenant_a_dir = genesis_dir(tmp.path(), "tenant-a");
    let tenant_b_dir = genesis_dir(tmp.path(), "tenant-b");
    std::fs::create_dir_all(&tenant_a_dir).unwrap();
    std::fs::create_dir_all(&tenant_b_dir).unwrap();

    let persona_a = name_to_persona_slug("Casey");
    let plan_a = plan(ROLE_DESC_A, &persona_a).expect("genesis plan A");
    write_vertical(&plan_a, &tenant_a_dir.join(&plan_a.vertical_id)).expect("write A");

    let persona_b = name_to_persona_slug("Maya");
    let plan_b = plan(ROLE_DESC_B, &persona_b).expect("genesis plan B");
    write_vertical(&plan_b, &tenant_b_dir.join(&plan_b.vertical_id)).expect("write B");

    let (reg_a, errs_a) = load_dir(&tenant_a_dir).unwrap();
    let (reg_b, errs_b) = load_dir(&tenant_b_dir).unwrap();
    assert!(errs_a.is_empty(), "{errs_a:?}");
    assert!(errs_b.is_empty(), "{errs_b:?}");

    // Tenant A's registry sees only A's vertical.
    let a_ids: Vec<&str> = reg_a.list().map(|(id, _)| id).collect();
    assert_eq!(a_ids, vec![plan_a.vertical_id.as_str()]);
    assert!(
        reg_a.get(&plan_b.vertical_id).is_none(),
        "tenant A leaked tenant B's vertical id"
    );

    // Tenant B's registry sees only B's vertical.
    let b_ids: Vec<&str> = reg_b.list().map(|(id, _)| id).collect();
    assert_eq!(b_ids, vec![plan_b.vertical_id.as_str()]);
    assert!(
        reg_b.get(&plan_a.vertical_id).is_none(),
        "tenant B leaked tenant A's vertical id"
    );
}

#[test]
fn genesis_subtree_is_invisible_to_root_load_dir_default_listing() {
    // The handler writes under `<verticals_root>/.genesis/<tenant>/`.
    // `load_dir` walks only the *immediate* subdirectories of its
    // argument — so a load_dir scoped at the root sees `.genesis`
    // as a single subdir with no `vertical.toml`, and silently
    // skips it. This mirrors what the gateway's catalog-listing
    // path (which walks `argo_vertical::default_dir()` at the
    // root) does, and is the load-bearing reason generated
    // verticals are NOT visible to `GET /api/v1/verticals`.
    let tmp = TempDir::new().unwrap();
    let tenant_dir = genesis_dir(tmp.path(), "tenant-a");
    std::fs::create_dir_all(&tenant_dir).unwrap();
    let persona = name_to_persona_slug("Casey");
    let plan_a = plan(ROLE_DESC_A, &persona).expect("genesis plan");
    write_vertical(&plan_a, &tenant_dir.join(&plan_a.vertical_id)).expect("write");

    let (root_registry, root_errors) = load_dir(tmp.path()).expect("load root");
    assert!(root_errors.is_empty(), "{root_errors:?}");
    assert_eq!(
        root_registry.len(),
        0,
        "root listing must not surface generated verticals"
    );
}
