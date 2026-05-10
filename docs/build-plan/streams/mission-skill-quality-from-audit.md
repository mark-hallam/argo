# Mission load-bearing wire — `QualityInputs::from_audit_history` constructor

**Owner**: claude. **Depends on**: `argo-skill::quality::QualityInputs` exists (verified at `crates/argo-skill/src/quality.rs:44`); `argo-audit` event surface stable.
**Authored against**: origin/main `cb0c0a90` on `2026-05-10`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-skill-quality-from-audit`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 4 item #77 ("Build `QualityInputs::from_audit_history` so quality-recompute runs as cron"). Today `QualityInputs` is constructed at scoring time from in-memory hand-built fixtures or test scaffolds. There is no path to derive `QualityInputs` from on-disk audit history — meaning the `compute_score(inputs)` primitive cannot be re-run later against the same skill to detect quality drift. Mission Claim 4 ("skills shared and learned") needs a deterministic offline recompute path so the forge / regression detector can run as a cron tick. The follow-up cron itself (#78) is a separate brief; this brief delivers the constructor.

## Scope

Add `QualityInputs::from_audit_history(skill_id, store: &AuditStore, since: Option<DateTime<Utc>>) -> ArgoResult<QualityInputs>` that walks the audit log for the named skill and aggregates:

- `runs_total` — count of `AuditKind::SkillExecuted` (or whichever kind exists today; verify) for the skill_id
- `runs_succeeded` — subset where outcome was Success
- `runs_failed` — subset where outcome was Failure
- `unique_callers` — distinct worker_id (or session_id) calling the skill
- `feedback_thumbs_up` / `feedback_thumbs_down` — count of `AuditKind::OperatorFeedback` rows linked to the skill via parent reference
- `latest_run_at` — max timestamp seen
- Other fields per the existing `QualityInputs` struct (verify shape in first 5 minutes)

If `since` is set, only audit rows after that timestamp count. If unset, walks all history. Returns `Ok(QualityInputs::default())` if no audit rows are found (does NOT error — a never-run skill has zero quality inputs, not an unknowable-quality skill).

This is NOT in scope: caching results; the cron tick that calls this (#78 follow-up); modifying `QualityScore` semantics; persisting derived inputs to a new table; cross-tenant aggregation.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-skill/src/quality.rs` — narrowly scoped: ADD `pub async fn from_audit_history(skill_id: &SkillId, store: &dyn AuditStoreReader, since: Option<DateTime<Utc>>) -> ArgoResult<QualityInputs>` impl method on `QualityInputs`. Use a trait-bound `AuditStoreReader` so tests can fake the store cheaply. NO change to existing `QualityInputs` fields, `Default`, `compute_score`, or `QualityScore`.
- `crates/argo-skill/src/quality_tests.rs` — narrowly scoped: append 6 new unit tests below the existing module: from-empty-audit-yields-default; from-audit-counts-success-failure; from-audit-counts-feedback; from-audit-with-since-filters-old-rows; from-audit-walks-cross-session-uniqueness; from-audit-handles-malformed-rows-gracefully (skip + log).
- `crates/argo-skill/src/audit_reader.rs` (NEW, sibling, ≤ 80 LOC) — defines `pub trait AuditStoreReader` with the narrow read surface needed by `from_audit_history`: `async fn rows_for_skill(skill_id: &SkillId, since: Option<DateTime<Utc>>) -> ArgoResult<Vec<AuditRow>>` (or similar — choose the smallest surface). Avoid pulling all of `argo-audit` into the public skill API.
- `crates/argo-skill/src/lib.rs` — narrowly scoped: ONE `pub mod audit_reader;` line + ONE `pub use audit_reader::AuditStoreReader;` re-export.
- `crates/argo-skill/Cargo.toml` — narrowly scoped: confirm `argo-audit` is in `[dependencies]` (it should be, transitively); add only if absent.
- `crates/argo-skill/tests/quality_from_audit_smoke.rs` (NEW) — integration smoke: seed an `argo_audit::AuditStore` with 10 rows for a synthetic skill, call `from_audit_history`, assert returned inputs match expectations.

## Files you must NOT touch

- `crates/argo-audit/**` — `AuditStore` is upstream contract; reuse, don't modify (the trait we add lives in `argo-skill`, narrowing what we need)
- `crates/argo-cli/handlers/skill_*.rs` — CLI wiring is a separate brief
- `crates/argo-store/migrations/**` — no schema change
- `crates/argo-skill/src/forge*.rs` — forge wires consume `QualityInputs` via `compute_score`; orthogonal here

## Acceptance criteria

- [ ] `QualityInputs::from_audit_history(skill_id, store, since)` exists and is async
- [ ] When the audit store has zero rows for the skill, returns `Ok(QualityInputs::default())` — does NOT error
- [ ] When `since` is `Some`, rows older than the cutoff are excluded from aggregation
- [ ] Success / failure counts match the count of `AuditKind::*` rows in the audit (specific kind names verified against current `argo_audit::AuditKind` at implementation time)
- [ ] Feedback thumbs counts derive from `AuditKind::OperatorFeedback` rows linked to the skill
- [ ] Unique callers derive from distinct `worker_id` (or session_id — pick whichever the audit row carries; document the choice)
- [ ] Malformed audit rows (e.g. missing JSON field that the parser expected) are skipped with a TRACE log line — do NOT propagate as error (one bad row should not fail recomputation)
- [ ] `AuditStoreReader` trait is the only new public abstraction; the trait surface is the minimum needed by `from_audit_history`
- [ ] 6 new unit tests in `quality_tests.rs` cover the matrix
- [ ] 1 integration smoke in `tests/quality_from_audit_smoke.rs` exercises end-to-end with a real `AuditStore`
- [ ] No regression in the 8+ existing `quality_tests.rs` tests
- [ ] `cargo build -p argo-skill` clean
- [ ] `cargo clippy -p argo-skill --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-skill` passes (skip pre-existing flakes per KNOWN_FLAKES.md)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-skill-quality-from-audit`
- PR title: `skill: QualityInputs::from_audit_history constructor for offline recompute (mission-skill-quality-from-audit)`

## Notes for the implementer

- First 10 minutes: read `crates/argo-audit/src/event.rs` to confirm the AuditKind variants this brief assumes (`SkillExecuted`, `OperatorFeedback`). If they're named differently, substitute and document. If neither exists today, BLOCK with reason "audit kind for skill execution not present; needs upstream `mission-audit-skill-execution-kind` brief".
- The `AuditStoreReader` trait pattern keeps the dependency narrow. `argo_audit::AuditStore` impls the trait via a small impl block (which can live in `argo-skill/src/audit_reader.rs` since that's a new module — no upstream change).
- Use `Default` semantics: `QualityInputs::default()` returns "perfect" inputs (per the existing test `let perfect = QualityInputs::default();`). That's intentionally optimistic; for the empty-history case we return `default()` which means a never-run skill scores high. If that's surprising, document in rustdoc — operators recompute periodically once usage data exists.
- The `since` parameter is for incremental recompute: a cron tick can pass `last_run_at` to skip already-processed history. v1 doesn't memoize — that's the cron brief #78.
- Don't introduce caching here. The cron tick (#78 follow-up) decides cadence; this brief just provides the deterministic constructor.
- If `AuditRow` carries a `payload: serde_json::Value` and the brief's expected fields aren't there, prefer skip-with-TRACE over panic. The audit log is append-only and historic; old rows may have a different schema.
