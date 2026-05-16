# codex-ci-workflows-reactivate — Re-enable CI/deny/audit/deploy-smoke workflows (F-01 P0)

**Owner**: codex. **Depends on**: nothing — but expect transient redness once active (use that to surface real regressions).
**Authored against**: origin/main `ad581eca` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-ci-workflows-reactivate`.
**Source**: F-01 P0 in `docs/skycot/audit/2026-05-16-one-shot-architecture-audit.md`.

## Scope

🔴 P0 — CI safety net is structurally off. 13 GitHub workflow files in `.github/workflows/` are checked in with `.disabled` suffix, so their triggers never fire:

```
audit.yml.disabled        deny.yml.disabled              real-llm-smoke.yml.disabled
bench.yml.disabled        deploy-smoke.yml.disabled      release.yml.disabled
ci.yml.disabled           docs.yml.disabled              schema-doc-check.yml.disabled
fuzz-nightly.yml.disabled oci-smoke.yml.disabled         silent-skip-nightly.yml.disabled
soak.yml.disabled
```

Only `cli-reference-drift.yml` is active. `MISSION_BACKLOG.md:75` marks CI re-enable as `[SHIPPED]` — the backlog is wrong; the executable gates are off.

This is the largest reality vs. ledger gap surfaced by the 2026-05-16 audit. Every other quality finding (LOC ceiling, duplicate types, function size, unsafe shims, smoke completeness) is enforceable in CI but currently un-enforced.

Scope (this brief — V1 SAFETY NET ONLY):

- Rename the 4 highest-value gates from `.disabled` to active: `ci.yml`, `deny.yml`, `audit.yml`, `deploy-smoke.yml`. These cover code-quality budget, supply-chain hygiene, security advisory check, and smoke pass.
- Leave the other 9 `.disabled` workflows alone — they're nightly/release/soak workflows that aren't v1-critical. Document in a follow-up brief if any need reactivation.
- Verify each reactivated workflow has the resource shape (runner, secrets, permissions) it needs on current main.
- If reactivation surfaces pre-existing CI redness (LOC violators per F-09, unsafe shims per F-14, duplicate types per F-08), DO NOT fix those here — author follow-up briefs. This brief's job is to TURN THE LIGHTS ON.

Operator GHA-billing context: GitHub Actions billing is currently failing per BUILD_STATUS.md header. Even after rename, workflows will not actually execute until billing is restored. The rename is still the right action — it makes the gates structurally active so they fire the moment billing resumes.

LOC: ~30 (mostly `git mv` operations + brief workflow YAML touch-ups to fix any path-staleness).

## Files you may touch

- `.github/workflows/ci.yml.disabled` → rename to `.github/workflows/ci.yml` via `git mv`. Verify trigger blocks are correct against current main branch name. ≤20 modified lines if YAML needs path/branch fixes.
- `.github/workflows/deny.yml.disabled` → rename. ≤10 modified lines.
- `.github/workflows/audit.yml.disabled` → rename. ≤10 modified lines.
- `.github/workflows/deploy-smoke.yml.disabled` → rename. Verify it invokes `scripts/smoke-skycot.sh` against a buildable argo binary. ≤20 modified lines.
- `docs/build-plan/MISSION_BACKLOG.md` — narrowly: update line ~75 to reflect actual state (CI re-enable was wrong as SHIPPED; flip to IN_PROGRESS or SHIPPED-this-brief). ≤5 modified lines.

## Files you must NOT touch

- The 9 other `.disabled` workflows (`bench`, `docs`, `fuzz-nightly`, `oci-smoke`, `real-llm-smoke`, `release`, `schema-doc-check`, `silent-skip-nightly`, `soak`). Out of scope.
- Source code under `crates/` — pre-existing CI violations are followups (F-09, F-08, F-14 have their own briefs).
- The `cli-reference-drift.yml` already-active workflow.

## Acceptance criteria

- [ ] 4 workflow files renamed: `ci.yml`, `deny.yml`, `audit.yml`, `deploy-smoke.yml` exist without `.disabled` suffix.
- [ ] `gh workflow list` shows them as active (after push).
- [ ] Each workflow's `on:` block has triggers that match current main branch name + sensible paths.
- [ ] MISSION_BACKLOG.md CI row reflects reality.
- [ ] BUILD_STATUS.md row updated with PR number.
- [ ] PR body documents which workflows are now active vs which were intentionally left disabled (and why).
- [ ] PR body explicitly notes any pre-existing redness that will surface (LOC budget violations, unsafe lint, etc.) — and flags the follow-up brief IDs.

## Branch + PR

- Branch: `codex-ci-workflows-reactivate`
- PR title: `ci: re-enable ci/deny/audit/deploy-smoke workflows (codex-ci-workflows-reactivate) — F-01 P0`

## Notes for the implementer

- This brief is **structural**, not bug-fixing. Don't try to make CI green during the reactivation. Just turn the gates on.
- After this lands, expect 1-2 follow-up briefs for whatever CI surfaces (likely F-09 LOC budget violations need fixing, possibly F-08 type-canon lint).
- Reactivating `deploy-smoke.yml` is critical because that's what gates `scripts/smoke-skycot.sh`. After it's active, F-10's macOS smoke fix gets continuous verification.
