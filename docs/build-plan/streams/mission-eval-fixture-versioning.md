# mission-eval-fixture-versioning — Eval fixture schema versioning + compat shim

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `66d07a69` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-eval-fixture-versioning`.

## Scope

Phase D audit P2 (smaller-scope subset of `mission-serialization-versioning`). Eval fixtures (under `verticals/*/eval/*.json` and `packs/*/evals/*.json`) currently have no schema-version field. Adding fields to the fixture schema later will silently break old fixtures.

This brief:
1. Adds an optional `schema_version: u32` field to the eval-fixture struct (default 1 when absent).
2. Adds a `Backward-compat` shim: legacy fixtures without the field deserialize as v1.
3. Documents the versioning policy + how to migrate fixtures forward.
4. Smoke-checks all shipped fixtures (`verticals/*/eval/`, `packs/*/evals/`) load cleanly under the new schema.

Smaller-scope variant of the larger `mission-serialization-versioning` brief (which targets all serialized types — that one is much bigger and stays in the post-v1 backlog).

LOC: ~250.

## Files you may touch

- `crates/argo-eval/src/fixture.rs` (or wherever the fixture struct is defined) — narrowly: add `#[serde(default = "default_schema_version")] schema_version: u32`. ≤30 modified lines.
- `crates/argo-eval/src/fixture_tests.rs` (sibling, if exists else NEW) — 4 unit tests: legacy load, v1 explicit load, v2 future-tagged load fails cleanly, default value. ≤80 modified lines.
- NEW `docs/skycot/operator/eval-fixture-versioning.md` — one-page policy doc. ≤80 modified lines.

## Files you must NOT touch

- The shipped fixture JSON files (no fixture content edits in this brief; schema-version is optional, legacy stays valid).
- argo-skill or argo-vertical (separate eval-consumer; not in scope).
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] `schema_version` field added to fixture struct, defaults to 1 if absent.
- [ ] All existing fixtures in `verticals/*/eval/` and `packs/*/evals/` load successfully without modification (test fixture loader smoke).
- [ ] Future-version (e.g., v2 unknown) fails with a clear error message, not a silent deserialization drop.
- [ ] 4 unit tests passing.
- [ ] Operator doc exists at `docs/skycot/operator/eval-fixture-versioning.md`.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `codex-mission-eval-fixture-versioning`
- PR title: `feat(eval): fixture schema versioning + compat shim (mission-eval-fixture-versioning)`

## Notes for the implementer

- Mission-claim alignment: production hardening (forward-compat for eval-driven claim 3 + claim 4 work).
- Smaller-scope subset of the post-v1 `mission-serialization-versioning` backlog item — that big brief stays separate.
- Drift-prevention: extends existing fixture surface; no parallel.
