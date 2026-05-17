# claude-audit-sink-lint-fix-and-wire-ci — false-positive fix + wire to CI (P2 audit-finding-9)

**Owner**: claude. **Sizing hint**: fast-mode candidate — bash script edit + CI workflow add.
**Depends on**: none.
**Authored against**: origin/main `6b432ec4` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-audit-sink-lint-fix-and-wire-ci`.
**Source**: 2026-05-17 audit finding #9 (lines 179-194). `scripts/lint-no-audit-sink-new-in-production.sh` exits 1 on a test helper (`routes_audit_feed_test_helpers.rs:23`) that's annotated `#![allow(deprecated)]`, and the lint is not wired into CI.

## Mission alignment

- **Security** (CLAUDE.md priority #4) — `AuditSink::new` is `#[deprecated]` for production. The intent is correct; the guard just has a blind spot.
- **Repeatability** (#3) — once the guard is reliable, wire it to CI as a non-bypassable gate.

## Scope

P2 fast-mode. Two parts:

### Part A: Fix the false positive

Current `scripts/lint-no-audit-sink-new-in-production.sh` only recognizes inline `#[cfg(test)] mod tests { ... }` patterns for excluding test code. It misses sibling helper files included via `#[path]` or named `*_test_helpers.rs`.

Teach the script:
1. Skip any file named `*_test_helpers.rs` or `*_tests.rs` (path-based suffix).
2. Skip any file with `#![allow(deprecated)]` at top-of-file (signals intentional test/legacy scope).
3. Skip any file inside a `tests/` directory (cargo integration-test root).
4. Continue to scan production `.rs` files for `AuditSink::new(` without `_with_tenant`.

Lint should report `OK: 0 production AuditSink::new callsites` on current main after the fix.

### Part B: Wire to CI

Add a new job `audit-sink-tenant-guard` in `.github/workflows/ci.yml`:

```yaml
audit-sink-tenant-guard:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Enforce AuditSink::new_with_tenant in production
      shell: bash
      run: bash scripts/lint-no-audit-sink-new-in-production.sh
```

Goes alongside the existing `prepared-statements` / `error-variant-usage` / `config-callsite-audit` jobs.

LOC: ~120.

## Files you may touch

- `scripts/lint-no-audit-sink-new-in-production.sh` (existing) — extend skip rules. ≤60 modified LOC.
- `scripts/lint-no-audit-sink-new-in-production_tests.sh` (NEW or extend existing) — golden tests for: production-violation, test-helper, integration-test, `#![allow(deprecated)]`. ≤40 LOC.
- `.github/workflows/ci.yml` — add the new job. ≤25 modified LOC.
- `docs/skycot/security/audit-sink-policy.md` (NEW or extend existing security/* doc) — operator runbook for the lint policy + how to add a documented exception. ≤30 LOC.

## Files you must NOT touch

- The audit chain code or `AuditSink::new_with_tenant` API — read-only consumer.
- `routes_audit_feed_test_helpers.rs` — the false-positive source. It's correct as-is; the LINT is wrong, not the file.
- Migration files — read-only.

## Acceptance criteria

- [ ] `bash scripts/lint-no-audit-sink-new-in-production.sh` exits 0 on current main.
- [ ] The lint reports `routes_audit_feed_test_helpers.rs` is SKIPPED (test helper) not FAILED.
- [ ] Lint also skips `crates/argo-audit/tests/` integration tests, `*_tests.rs` siblings, and files with `#![allow(deprecated)]` at top.
- [ ] Lint still flags a synthetic production violation (test fixture: a `.rs` file under `crates/argo-cli/src/handlers/` with `AuditSink::new(store)` should FAIL).
- [ ] New `audit-sink-tenant-guard` CI job runs on PR + push and passes on current main.
- [ ] Operator runbook documents: when to add an exception, how to add one, why this lint exists.
- [ ] `bash scripts/lint-no-audit-sink-new-in-production_tests.sh` passes (4 golden cases).
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `claude-audit-sink-lint-fix-and-wire-ci`
- Title: `chore(ci): fix audit-sink-tenant-guard false positive + wire to CI (claude-audit-sink-lint-fix-and-wire-ci)`
- Worker verification: lint test script + CI workflow YAML parse + per SWARM_PROTOCOL.md.

## Hidden constraints

- **Don't suppress all `#[deprecated]` warnings.** Just skip files that explicitly opt-in to `#![allow(deprecated)]` at the file level. Per-function `#[allow(...)]` does NOT count as an opt-out.
- **`*_test_helpers.rs` is a convention,** not a hard rule. Verify with `rg --type rust -l 'AuditSink::new' crates/` and visually confirm each result is either production or correctly-skipped.
- **Lint is fail-closed.** If parsing logic is ambiguous, prefer false-positive over false-negative. Operator can always add a documented exception.
- **Closes audit finding #9.** When this merges, the audit's #9 (false-positive + not-in-CI) is resolved.
