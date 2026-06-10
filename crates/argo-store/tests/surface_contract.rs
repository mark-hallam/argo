//! Public-surface contract pins for `argo-store`.
//!
//! Authored by `pin-surface-contract-argo-types-store-audit`
//! (test-health audit 2026-06-10, F4). `argo-store` is the database
//! heart: the SQLite session store every persistence path routes
//! through. It carries 17 integration-test files but had no surface
//! lock, so a silent change to a `SessionStore::open*` constructor, the
//! `migrate` entry point, the `EXPECTED_SCHEMA_VERSION` health-check
//! constant, or the `AuditRow` row shape could drift unnoticed. These
//! import + type-level fences make the store's load-bearing entry
//! points and canonical row structs fail loud at compile time.

#![allow(dead_code)]

// 1. Re-export inventory fence — fails to COMPILE if any canonical
//    store symbol is renamed or dropped from `lib.rs`.
use argo_store::{
    AuditRow, DbStats, ForgeRunRow, FreezeRecord, GlobalSpend, JudgeRow, MarketplaceArtifactRow,
    MessageRow, MessageSearchHit, RecoveryCacheEntry, SessionCreate, SessionGrant, SessionHeader,
    SessionStore, SkillRunRow, StoreResult, WorkerId, EXPECTED_SCHEMA_VERSION,
};
use argo_types::SessionId;

// ---------------------------------------------------------------------------
// 2. Store entry points: constructor + migration + schema-version
//    signatures. The gateway startup health check compares
//    `schema_version()` against `EXPECTED_SCHEMA_VERSION` and refuses
//    to bind on divergence, so both must stay wired.
// ---------------------------------------------------------------------------

#[test]
fn store_entry_point_constants_are_stable() {
    // `EXPECTED_SCHEMA_VERSION` is the health-check sentinel; pin its
    // type (a monotonically-bumped `u32`). The gateway refuses to bind
    // when this diverges from the on-disk `schema_version`; the live
    // value is asserted against a freshly-migrated store in
    // `open_migrate_and_schema_version_round_trip`.
    let _: u32 = EXPECTED_SCHEMA_VERSION;

    // `StoreResult<T>` resolves to `Result<T, _>` — typed-binding fence.
    let _ok: StoreResult<u8> = Ok(7);
}

#[tokio::test]
async fn open_migrate_and_schema_version_round_trip() {
    // The end-to-end contract the whole workspace depends on: open an
    // in-memory store, migrate, and read back the live schema version.
    let store = SessionStore::open(":memory:")
        .await
        .expect("open in-memory store");
    store.migrate().await.expect("migrate");
    let version = store.schema_version().await.expect("schema_version");
    assert_eq!(
        version, EXPECTED_SCHEMA_VERSION,
        "freshly-migrated store must report the binary's expected schema version"
    );
}

// ---------------------------------------------------------------------------
// 3. Canonical row structs — the read-side shapes consumers
//    deserialize. `AuditRow` is the highest-stakes (it was duplicated
//    across 4 crates pre-canonicalization, a real data-integrity
//    risk), so pin its exact field set.
// ---------------------------------------------------------------------------

#[test]
fn audit_row_field_shape_is_stable() {
    let row = AuditRow {
        seq: 1,
        kind: "tool_call".into(),
        payload_json: "{}".into(),
        created_at: 0,
    };
    // Field-type fences: a renamed/retyped column breaks compilation.
    let _: i64 = row.seq;
    let _: String = row.kind;
    let _: String = row.payload_json;
    let _: i64 = row.created_at;
}

#[test]
fn session_header_core_fields_are_stable() {
    // `SessionHeader` carries the canonical id triple
    // (session/tenant/workspace) plus model/provider. Pin the types of
    // the load-bearing identity columns.
    let _: fn(&SessionHeader) -> SessionId = |h| h.id;
    let _: fn(&SessionHeader) -> &str = |h| h.model.as_str();
    let _: fn(&SessionHeader) -> &str = |h| h.provider.as_str();
}

#[test]
fn worker_id_constructors_are_stable() {
    let _: fn() -> WorkerId = WorkerId::new;
    let _: fn() -> WorkerId = WorkerId::nil;
    // `from_blob` decodes a stored UUID blob; arity + result fence.
    let _: fn(&[u8]) -> Result<WorkerId, argo_types::ArgoError> = WorkerId::from_blob;
    assert_ne!(WorkerId::new(), WorkerId::nil());
}

// ---------------------------------------------------------------------------
// 4. Canonical row / value-type inventory. These are the read-side
//    shapes consumers deserialize across the workspace; naming each
//    makes a rename or removal from the re-export set a compile error.
// ---------------------------------------------------------------------------

#[test]
fn canonical_store_row_types_are_named() {
    let _: Option<DbStats> = None;
    let _: Option<ForgeRunRow> = None;
    let _: Option<FreezeRecord> = None;
    let _: Option<GlobalSpend> = None;
    let _: Option<JudgeRow> = None;
    let _: Option<MarketplaceArtifactRow> = None;
    let _: Option<MessageRow> = None;
    let _: Option<MessageSearchHit> = None;
    let _: Option<RecoveryCacheEntry> = None;
    let _: Option<SessionGrant> = None;
    let _: Option<SkillRunRow> = None;
    let _: Option<SessionCreate<'static>> = None;
}
