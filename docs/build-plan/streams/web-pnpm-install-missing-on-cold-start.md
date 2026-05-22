# web-pnpm-install-missing-on-cold-start

**Owner**: claude.
**Depends on**: none.
**Authored against**: `origin/main` `d2be56888` on `2026-05-22`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-pnpm-install-missing-on-cold-start`.

🟢 **P2 SMOKE-GATE COVERAGE.** Source: evaluator bootstrap 2026-05-22T02:25 side-finding + auditor T03-39 §4 P2. `@radix-ui/react-tooltip@1.2.8` is declared in `web/package.json` but absent from `web/node_modules`. The CI smoke gate didn't catch it because the gate runs against a warm cache. A fresh checkout on a new machine fails at `pnpm dev` with a missing-module error.

## Scope

Add a `web-cold-install` GitHub Actions workflow (or step in the existing smoke workflow, if one exists) that:
1. Checks out the repo.
2. Removes any cached `node_modules` and pnpm store.
3. Runs `pnpm install --frozen-lockfile` from a cold cache.
4. Runs `pnpm typecheck` and `pnpm lint` to confirm a usable dev environment.

This guarantees that any future PR adding a dependency to `package.json` without updating `pnpm-lock.yaml` (or adding a dependency that's referenced but not declared) is caught before merge.

Plus a one-time fix for the current `@radix-ui/react-tooltip` gap: run `pnpm install` locally, confirm the lockfile updates, commit the lockfile change in this PR.

The auditor flagged this as P2; the operator may prefer to defer until GHA billing is restored (per BUILD_STATUS.md header: "CI re-enable (P1) is deferred until GitHub Actions billing restored"). If the operator has not restored GHA billing by the time this brief is claimed, the workflow file can land disabled (`on: workflow_dispatch:` only) so the policy is in the repo even if it can't run yet — note the disabled state in the PR body.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `NEW: .github/workflows/web-cold-install.yml` (or merged into an existing workflow if one logically covers the same gate).
- `web/pnpm-lock.yaml` — narrowly scoped: regenerate via `pnpm install` to add the `@radix-ui/react-tooltip` entry. No other lockfile churn.
- `web/package.json` — narrowly scoped: ONLY if the install reveals a version-range tweak. Do not edit otherwise.
- `.github/workflows/DISABLED_WORKFLOWS_POLICY.md` — narrowly scoped: append a one-line note if the new workflow lands disabled (per the existing policy doc convention).

## Files you must NOT touch

- Any application source code under `web/app/`, `web/lib/`, `web/components/`.
- Any other GHA workflow.
- `BUILD_STATUS.md` outside the standard row flip.

## Acceptance criteria

- [ ] `.github/workflows/web-cold-install.yml` exists. ~50 LOC.
- [ ] Workflow removes any pnpm cache + runs `pnpm install --frozen-lockfile` cold.
- [ ] Workflow runs `pnpm typecheck` and `pnpm lint` after the cold install.
- [ ] `web/pnpm-lock.yaml` is regenerated to include `@radix-ui/react-tooltip` (the regression we're closing).
- [ ] After the fix, a clean `rm -rf web/node_modules web/.pnpm-store && pnpm -C web install --frozen-lockfile` succeeds end-to-end (document in PR body).
- [ ] PR body states clearly whether the workflow is `on: pull_request` (live) or `on: workflow_dispatch` (parked until GHA billing restored).
- [ ] If parked, an entry is appended to `.github/workflows/DISABLED_WORKFLOWS_POLICY.md`.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-web-pnpm-install-missing-on-cold-start`
- PR title: `ci(web): cold-install gate + restore missing radix-tooltip entry (web-pnpm-install-missing-on-cold-start)`

## Notes for the implementer

- ~50-100 LOC total. The workflow file is small; the lockfile regeneration is automated.
- Use `actions/setup-node@v4` + a separate `pnpm/action-setup@v2` (or whichever versions the existing workflows use — match, don't invent).
- The `--frozen-lockfile` flag is the load-bearing part: it FAILS if `package.json` declares a dep not in the lockfile, which is exactly the regression class we're catching.
- Check whether `web/.gitignore` excludes `node_modules` and `.pnpm-store` (it should). If not, add to gitignore in a follow-up — not in scope here.
- If the regenerated lockfile shows churn beyond adding `@radix-ui/react-tooltip` (e.g. a sibling version bump), STOP and mark BLOCKED with reason "lockfile churn beyond scope — needs targeted regenerate". Don't push a lockfile that updates 30 packages at once.
