# ops-document-or-remove-legacy-sql-migrations-directory — clarify the two-migration-system landscape so future agents don't false-alarm on tip mismatch

**Owner**: claude or codex. **Depends on**: nothing.
**Authored against**: origin/main `d5b7cc458` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-document-or-remove-legacy-sql-migrations-directory`.

## Why this is P3 (round-71 — disambiguation, prevents recurring false alarms)

Evaluator round-71 surfaced a confusion source while verifying PR #2334 (`ops-add-expected-schema-version-vs-tip-migration-ci-lint`): **two migration systems exist side-by-side in `crates/argo-store/`**, and at first glance each appears authoritative for "tip migration".

System A — **`crates/argo-store/src/migrations/v*.rs`** (Rust modules):
- Tip: `v83.rs` (latest).
- Drives `EXPECTED_SCHEMA_VERSION` in `crates/argo-store/src/lib.rs`.
- What the running binary applies at startup.
- The lint PR #2334 added pins this against `EXPECTED_SCHEMA_VERSION`.

System B — **`crates/argo-store/migrations/NN__name.sql`** (numbered SQL files):
- Tip: `109__stripe_customer.sql` (highest slot number, total 105 files).
- Read at compile time by `crates/argo-store/build.rs` and embedded as `MIGRATION_FILES` for the duplicate-slot validator (`crates/argo-store/src/migrations/duplicate_slot_validator.rs`).
- **Not** what the running binary applies — System A is.
- Highest slot (`109__`) is purely for slot-collision detection, NOT a schema version number.

Evaluator round-71's initial alarm (`tip=99` vs `EXPECTED=83`) was a false positive caused by reading System B's filename slot as a schema version. Once disambiguated, both systems are working as designed — but the confusion will recur every time a fresh agent (or operator) audits the migration story.

This brief makes that disambiguation discoverable from inside the codebase, not requiring tribal knowledge or an evaluator round to surface.

Round-71 source: `docs/skycot/evals/scheduled/2026-05-25/round-71-pr-2334-schema-lint-pre-merge-pass.md` (System A vs System B distinction).

## Scope

Three small docs additions + one assertion. **Zero behavioural changes.**

1. **Add a top-of-directory README** at `crates/argo-store/migrations/README.md` (~20 lines) explaining: this directory is the **slot-collision corpus** for `build.rs`, not the schema-apply path; the schema-apply path lives at `crates/argo-store/src/migrations/v*.rs`; the highest slot number here is **not** a schema version and does NOT control what the binary applies.

2. **Add a top-of-directory README** at `crates/argo-store/src/migrations/README.md` (~20 lines) explaining: this directory is the **schema-apply path** (versioned Rust modules); tip module → `EXPECTED_SCHEMA_VERSION`; the sibling `crates/argo-store/migrations/` SQL directory is unrelated and exists only for slot-collision detection.

3. **Update `crates/argo-store/build.rs:1-4`** to add a one-sentence pointer in the existing doc comment: "Note: this is the slot-collision corpus, not the schema-apply path. See `src/migrations/README.md` for the schema-apply story."

4. **Add an entry to `docs/skycot/operator/migrations.md`** (the existing migrations doc) that links to both READMEs and states the two-directory contract.

**Not in scope**: removing either directory, refactoring `build.rs`, changing the duplicate-slot validator, changing `EXPECTED_SCHEMA_VERSION` semantics, modifying any `.sql` or `v*.rs` file, behavioural changes of any kind.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/migrations/README.md` — new file.
- `crates/argo-store/src/migrations/README.md` — new file.
- `crates/argo-store/build.rs` — single-line doc comment addition (no logic change).
- `docs/skycot/operator/migrations.md` — additive section near top.

## Files you must NOT touch

- Any `.sql` file under `crates/argo-store/migrations/`.
- Any `v*.rs` file under `crates/argo-store/src/migrations/`.
- `crates/argo-store/src/lib.rs` (no `EXPECTED_SCHEMA_VERSION` change).
- The duplicate-slot validator code.
- Anything that compiles to a behavioural difference.

## Acceptance criteria

- [ ] `cargo check -p argo-store` exits `0` (no behaviour change).
- [ ] Both README files exist with the explanatory content above.
- [ ] `crates/argo-store/build.rs` doc comment includes the one-sentence pointer.
- [ ] `docs/skycot/operator/migrations.md` includes the two-directory section with cross-links to both READMEs.
- [ ] Manual verification: a fresh reader landing on either directory can find the other directory's purpose without grepping unrelated files.
- [ ] BUILD_STATUS.md updated with PR number on flip to IN_REVIEW.

## Branch + PR

- Branch: `claude-ops-document-or-remove-legacy-sql-migrations-directory` or `codex-ops-document-or-remove-legacy-sql-migrations-directory`.
- PR title: `docs(store): disambiguate slot-collision corpus from schema-apply path (ops-document-or-remove-legacy-sql-migrations-directory)`.

## Notes for the implementer

- Docs-only. Zero LOC of code change. Falls under overnight `--skip-clippy` ops-* auth.
- Fast-mode candidate (~10-15 min total).
- The "or-remove" half of the brief title is intentional: a future operator may decide System B is redundant and can be derived from System A. But that's a bigger blast radius (the duplicate-slot validator would need rewiring) — **do NOT do that in this brief.** Documentation first; consolidation only after operator sign-off.
- Reference: `docs/skycot/evals/scheduled/2026-05-25/round-71-pr-2334-schema-lint-pre-merge-pass.md` for the System A / System B distinction in evaluator's own words.
