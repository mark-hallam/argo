# Stream 0 — CI cleanup (preflight)

**Note**: this brief is included in the landing PR that ships
the build plan, so by the time you read this it should already
be `MERGED`. If it isn't, claim it.

## Scope

Disable the GitHub Actions workflows that auto-fire on every
PR but never run successfully (billing-blocked at the account
level). Workflows are preserved for revival once billing is
sorted; only the auto-trigger is removed.

## Files you may touch
- `.github/workflows/ci.yml`
- `.github/workflows/docs.yml`
- `.github/workflows/oci-smoke.yml`

## Files you must NOT touch
- `.github/workflows/release.yml` (tag-only; doesn't fire on
  PRs)
- Any other file

## Acceptance criteria
- [ ] Each of the three workflows has its `on:` trigger reduced
      to `workflow_dispatch:` only — no `push:`, no
      `pull_request:`
- [ ] A comment at the top of each modified file explains why
      auto-trigger is disabled and how to revive (one line:
      "auto-trigger disabled while billing is blocked; revive
      by restoring the original `on:` block")
- [ ] `release.yml` is unchanged
- [ ] After this PR, opening a fresh test PR fires zero CI jobs
      (verify via `gh pr checks`)

## Branch + PR
- Branch: `claude-0-ci-cleanup` (Claude does this in the
  landing PR; if Codex picks it up later, use
  `codex-0-ci-cleanup`)
- PR title: `ci: disable auto-trigger on noisy workflows
  pending billing fix (0-ci-cleanup)`

## Reference

Recent CI runs from `gh run list --limit 20`: 19 of 20 are
failures (billing-blocked). Operator policy is "every PR's code
is verified locally before merge." So zero value from the
auto-firing workflows; pure noise.
