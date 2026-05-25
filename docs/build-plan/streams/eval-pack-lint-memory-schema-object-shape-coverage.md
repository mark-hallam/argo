# Stream eval-pack-lint-memory-schema-object-shape-coverage — extend pack-lint to validate memory-schema object shape

**Owner**: codex (lint/schema work in Rust; ~80-120 LOC).
**Depends on**: none.
**Authored against**: origin/main `be9206ef2` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-pack-lint-memory-schema-object-shape-coverage`.

🟢 **P3.** Source: round-51 evaluator finding (#2259 M22 Status Reporter pack pre-merge verify), 2nd-consecutive round carry-forward. PR #2259's pack landed cleanly but the evaluator noted that the pack-lint validates pack-manifest top-level shape without validating the `memory_schema` sub-object's shape — a pack with malformed memory_schema (missing required fields, wrong types) would pass lint and only fail at runtime when an agent tries to bind memory.

## Scope

Find the pack-lint code (likely `crates/argo-vertical-packs/src/lint.rs` or similar — grep `pub fn lint_pack` to locate). Extend its validation to walk the `memory_schema` JSON object and assert:

1. Top-level keys present: `version` (string semver), `tables` (array of table-descriptor objects).
2. Each table descriptor has: `name` (string), `columns` (array of column objects), optional `primary_key` (array of column-name strings).
3. Each column object has: `name` (string), `type` (one of the supported set: `string`, `int`, `float`, `bool`, `json`, `datetime`), optional `nullable` (bool, defaults true), optional `default` (any).
4. No two columns in the same table have duplicate `name`.
5. `primary_key` values, if present, must reference existing column names in the same table.

Failing any check should produce a clear lint error citing the pack name, table name, and field. Style after existing pack-lint error messages.

## Grep before write

```
rg -n "fn lint_pack|pack_lint|memory_schema" crates/argo-vertical-packs/ crates/argo-packs/ crates/argo-cli/src/handlers/
```

If the schema validator already exists for another shape (e.g. workflow-step schema), prefer extending its shared validator rather than authoring a parallel one.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: [pack-lint module]

## Files you may touch

- The pack-lint module (TBD via grep).
- Any test fixture pack used in lint tests — add a small fixture for each new invalid-memory-schema case.
- Add 5+ unit tests, one per validation rule above.

## Tests

- 5 unit tests (one per rule, each with a fixture exercising the failure).
- 1 positive test: a well-formed memory_schema passes lint.

## Definition of done

- Pack-lint rejects malformed `memory_schema` with clear errors.
- `cargo test -p argo-vertical-packs --test pack_lint` (or wherever the lint tests live) green.
- PR body includes sample fixture diff + a sample error message output.
- The M22 Status Reporter pack from #2259 passes the extended lint (regression-safe).

Notes: defensive-quality work, not demo-blocking. Strictly additive; do not change pack-loading runtime behavior.
