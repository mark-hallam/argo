# ops-add-expected-schema-version-vs-tip-migration-ci-lint — CI lint that catches `EXPECTED_SCHEMA_VERSION` lag behind tip migration

**Owner**: claude or codex. **Depends on**: nothing.
**Authored against**: origin/main `24cb565b8` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-add-expected-schema-version-vs-tip-migration-ci-lint`.

## Why this is P3 (preventive — round-67/68/69 evaluator β)

The exact failure mode that took the gateway down on 2026-05-25 (#2317 added `v83.rs` + registered it but forgot to bump `EXPECTED_SCHEMA_VERSION` in `crates/argo-store/src/lib.rs:364`) is repeatable and catchable by static lint. Round-67/68/69 evaluator names this `β` and recommends a ~30 LOC bash check sibling to `scripts/check-migration-slots.sh`.

Pattern: every `crates/argo-store/src/migrations/vN.rs` file declares migration slot `N`. The highest such `N` is the "tip". `crates/argo-store/src/lib.rs:364` defines `pub const EXPECTED_SCHEMA_VERSION: u32 = N;` — that constant MUST equal the tip. When it lags, the gateway boots, `verify_schema_version` fails post-migration, /health returns 503. A 1-line bash script can grep both and fail CI with a clear remediation message.

Round-67/68/69 source: `docs/skycot/evals/scheduled/2026-05-25/round-67-pr-2317-bump-expected-schema-version-post-merge.md` (the original finding) and round-69 (`round-69-pr-2331-expected-schema-version-merged-but-gateway-binary-stale.md`) recommending this β as a preventive ci-lint.

## Scope

New file `scripts/check-expected-schema-version-matches-tip.sh` — bash, ~30 LOC, modeled on `scripts/check-migration-slots.sh`. Behaviors:

1. Find the tip migration via `ls crates/argo-store/src/migrations/v*.rs | sort -V | tail -1` → extract `N` via `grep -oE 'v[0-9]+'`.
2. Grep `crates/argo-store/src/lib.rs` for `pub const EXPECTED_SCHEMA_VERSION: u32 = (\d+);` → extract `M`.
3. If `M != N`, print `❌ EXPECTED_SCHEMA_VERSION=M lags behind tip migration vN. Bump crates/argo-store/src/lib.rs:<line> to N. (This is the v83 pattern from round-67/68/69 — see docs/skycot/evals/scheduled/2026-05-25/round-67-pr-2317-bump-expected-schema-version-post-merge.md.)` and exit `1`.
4. Else print `✅ EXPECTED_SCHEMA_VERSION=N matches tip migration vN.` and exit `0`.

Wire into the pre-merge gate (`scripts/conductor-pre-merge-gate.sh`) as a fast pre-flight check before gate-1 (cargo check). This is bash-only, sub-second, no cargo.

Also add a row to `.github/workflows/` (if any active workflows run lint scripts) OR document that the gate-script run is the enforcement point. Per `.github/workflows/DISABLED_WORKFLOWS_POLICY.md`, GHA is operator-blocked — so the gate-script wire is the load-bearing enforcement until GHA returns.

**Not in scope**: refactoring the schema-version check at runtime, adding a runtime-level invariant test (already exists at `verify_schema_version`), touching `check-migration-slots.sh`.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `scripts/check-expected-schema-version-matches-tip.sh` — new file.
- `scripts/conductor-pre-merge-gate.sh` — add one-line invocation in the pre-flight block.
- `scripts/check-expected-schema-version-matches-tip-tests.sh` — small test harness (optional but encouraged — mirrors `check-gateway-freshness-tests.sh`).

## Files you must NOT touch

- Any `crates/argo-store/src/migrations/vN.rs` file.
- `crates/argo-store/src/lib.rs` — the constant is correct as of #2331.

## Acceptance criteria

- [ ] `bash scripts/check-expected-schema-version-matches-tip.sh` exits `0` on current main (`EXPECTED_SCHEMA_VERSION=83` matches `v83.rs` tip).
- [ ] Adding a stub `crates/argo-store/src/migrations/v84.rs` (then reverting) demonstrates the script exits `1` with the expected error message.
- [ ] `bash scripts/conductor-pre-merge-gate.sh` runs the new check as part of pre-flight and surfaces it in its output.
- [ ] BUILD_STATUS.md updated with PR number on flip to IN_REVIEW.

## Branch + PR

- Branch: `claude-ops-add-expected-schema-version-vs-tip-migration-ci-lint` or `codex-ops-add-expected-schema-version-vs-tip-migration-ci-lint`.
- PR title: `ops: lint that catches EXPECTED_SCHEMA_VERSION lag behind tip migration (ops-add-expected-schema-version-vs-tip-migration-ci-lint)`.

## Notes for the implementer

- Use shellcheck-clean bash. Pattern after `scripts/check-migration-slots.sh` for style.
- Migration slot detection: `ls crates/argo-store/src/migrations/v*.rs | sort -V | tail -1 | grep -oE 'v[0-9]+' | head -1`. Already used in `check-gateway-freshness.sh:60-64`.
- Constant extraction: `grep -oE 'EXPECTED_SCHEMA_VERSION:\s*u32\s*=\s*[0-9]+' crates/argo-store/src/lib.rs | grep -oE '[0-9]+' | head -1`.
- Keep error message specific so the next person hitting it knows the exact file + line to fix.
- Fast-mode candidate. ~30 LOC bash + ~30 LOC tests.
