# Stream Audit-P2-1 — canonicalize duplicate `BundleFile` + `LifecycleState` public types

**Owner**: codex.
**Depends on**: none.
**Authored against**: origin/main `d2e4a579` on `2026-05-19`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-audit-bundlefile-canonicalize`.
**Source**: 2026-05-19 codex audit P2-2, validated by claude `VALIDATION.md` (CONFIRMED — `lint-parallel-pub-types.py` reports `BundleFile` in `argo-identity/src/data_export.rs:62` and `argo-replay/src/bundle.rs:38`) **PLUS a second collision introduced 2026-05-19 by #1846**: `LifecycleState` in `argo-skill/src/skill.rs:134` and `argo-store/src/workflow_definition.rs:91`.

## Scope (AMENDED 2026-05-19 by conductor)

`scripts/lint-parallel-pub-types.py` reports TWO duplicate public types on current main:

```
- BundleFile: crates=argo-identity, argo-replay; unallowed_pairs=argo-identity/argo-replay
  struct argo-identity   crates/argo-identity/src/data_export.rs:62
  struct argo-replay     crates/argo-replay/src/bundle.rs:38

- LifecycleState: crates=argo-skill, argo-store; unallowed_pairs=argo-skill/argo-store
  enum   argo-skill      crates/argo-skill/src/skill.rs:134
  enum   argo-store      crates/argo-store/src/workflow_definition.rs:91
```

This brief canonicalizes BOTH in one consistent pass so the lint exits 0 after merge. They are independent rename operations, but bundling them is cheaper than two PRs and prevents drift where the lint stays red between merges.

### Duplicate 1 — `BundleFile`

Two structs with the **same name, different shapes**:
- `argo-identity::BundleFile` — `(path, contents: String)` — used by data-export tenant-bundle assembly.
- `argo-replay::BundleFile` — `(path, bytes: Vec<u8>, is_json)` — used by replay bundle export.

Pure rename + caller updates. Genuine accidental collision (not two views of one concept).

### Duplicate 2 — `LifecycleState`

Two enums with the **same name, overlapping but non-identical variant sets** (introduced 2026-05-19 when #1846 mission-pm-workflow-persistence-schema merged):
- `argo-skill::LifecycleState` (older — pre-existing) — 5 variants: `Draft`, `PendingReview`, `Active`, `Rejected`, `Archived`. Methods `label()` / `from_label()`. Skill-marketplace human-approval lifecycle.
- `argo-store::LifecycleState` (new) — 3 variants: `Draft`, `Active`, `Archived`. Methods `as_str()` / `parse()`. Workflow-definition versioning lifecycle (no human-approval gate).

The two domains are genuinely different (skill marketplace vs workflow versioning). The right canonicalization is **rename the newer one to a domain-specific name**, leaving the existing skill consumers untouched. The newer (argo-store) was added 2026-05-19 with only one consumer (`workflow_definition.rs` + its tests + the mod registration); the older (argo-skill) has many consumers across the skill marketplace, store, and runner.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

### BundleFile half

- `crates/argo-identity/src/data_export.rs` — rename `pub struct BundleFile` → `pub struct DataExportBundleFile`. Update the constructor, the field accessors if any, and the impl blocks.
- `crates/argo-replay/src/bundle.rs` — rename `pub struct BundleFile` → `pub struct ReplayBundleFile`. Same treatment.
- All callers of the two types — find via `rg "BundleFile" crates/ --type rust` (filter out test fixtures that just contain the word "Bundle"). Most callers will be in the same crate; cross-crate uses are limited.
- `crates/argo-identity/src/lib.rs` + `crates/argo-replay/src/lib.rs` — if either crate re-exports `BundleFile` at the crate root, update the re-export to the new name. Add a `#[deprecated]` type alias `pub type BundleFile = DataExportBundleFile;` (and the replay equivalent) if external consumers might exist; flag for removal in next major.
- `crates/argo-replay/src/bundle_tests.rs` and `crates/argo-identity/src/data_export_tests.rs` (if they exist) — update test code to use the new names.

### LifecycleState half

- `crates/argo-store/src/workflow_definition.rs` — rename `pub enum LifecycleState` → `pub enum WorkflowLifecycleState`. Update `impl WorkflowLifecycleState`, the field type on the `WorkflowDefinition` struct, and the `as_str()` / `parse()` methods.
- `crates/argo-store/src/workflow_definition_tests.rs` — update test fixtures using `LifecycleState::Draft` → `WorkflowLifecycleState::Draft`.
- Any caller of `argo_store::workflow_definition::LifecycleState` — find via `rg "workflow_definition::LifecycleState|LifecycleState" crates/ --type rust | grep -v argo-skill` to scope to workflow-side consumers. As of d2e4a579, this is intra-crate only (workflow CRUD is fresh from #1846).
- DO NOT touch `crates/argo-skill/src/skill.rs::LifecycleState` or its consumers — argo-skill keeps the unqualified `LifecycleState` name because (a) it predates the collision and (b) it has many more consumers across the skill marketplace.

## Files you must NOT touch

- `scripts/lint-parallel-pub-types.py` or the allowlist — the lint is correct; the code is wrong.
- Any file that mentions `"BundleFile"` or `"LifecycleState"` only as a string literal (audit messages, doc comments referring to the type by quoted name, SQL `lifecycle_state` column references) — leave the strings alone if they describe data, not the Rust type.
- `crates/argo-skill/src/skill.rs::LifecycleState` — keep the established name; only argo-store's newer one renames.
- Anything in `web/` — these are Rust-internal types.

## Acceptance criteria

- [ ] `python3 scripts/lint-parallel-pub-types.py` exits 0 (NO duplicate names reported — both BundleFile AND LifecycleState resolved).
- [ ] Both `BundleFile` renames compile + all callers updated.
- [ ] `argo-store::workflow_definition::WorkflowLifecycleState` compiles; `workflow_definition_tests.rs` passes with the new name.
- [ ] `argo-skill::LifecycleState` unchanged; skill consumers compile without edits.
- [ ] If `#[deprecated]` aliases are added (BundleFile half only — LifecycleState half is too new for aliases to matter), they emit warnings only — no `-D warnings` regression (use `#[deprecated(note = "...")]` not `#[deny(deprecated)]`).
- [ ] Public API rustdoc is updated to reflect new names; example snippets compile.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-identity -p argo-replay -p argo-store -p argo-skill` passes.
- [ ] Every modified file < 500 LOC (the existing files are well under; this should not approach the limit).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `codex-mission-audit-bundlefile-canonicalize`
- PR title: `types: canonicalize BundleFile + LifecycleState parallel duplicates (mission-audit-bundlefile-canonicalize)`

## Notes for the implementer

- This is THE last duplicate public-type set in the workspace (post-bundle: lint exits 0). After this lands, the lint stays green forever — please add a one-line note to BUILD_STATUS.md under "✅ Closed" that the parallel-pub-types lint is now permanently green.
- The two renames are independent — work on them in either order. If something goes wrong with one, ship the other half as a separate sub-PR rather than reverting both.
- `WorkflowLifecycleState` is the recommended name for argo-store; alternatives (`WorkflowState`, `WorkflowVersionLifecycle`) are also acceptable if they read better in context — exercise judgment, but state the choice in the PR body.
- Cross-crate dependency direction: `argo-replay` and `argo-identity` are siblings (sibling rename safe); `argo-store` depends on `argo-skill` indirectly via re-exports — verify no cycle is introduced by `cargo metadata`.
- LifecycleState half is internal to argo-store and the `pub` is consumed only within the crate as of d2e4a579 — no deprecated alias needed (no external consumers to keep working).

## Out of scope (follow-up briefs)

- Removing the deprecated `BundleFile` aliases — separate PR after a release cycle.
- Broader naming-consistency sweep across `argo-replay` and `argo-identity` — premature; two renames are two renames.
- Lifting `BundleFile` shapes into a shared `argo-types` crate as a single trait — overengineering for two non-overlapping use cases.
- Merging the two `LifecycleState` enums in argo-types — overengineering; the skill and workflow lifecycles are genuinely different concerns.
