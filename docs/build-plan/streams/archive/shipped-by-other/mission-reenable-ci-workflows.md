# Mission production blocker #1 — re-enable GitHub Actions CI

**Owner**: codex. **Depends on**: none. **PRIORITY: ship-blocker for any external trust.**

> SOURCE: deep-research findings tick 198, Section 2, item #9.
> CI is currently `workflow_dispatch:` only — no automated
> regression detection. SOC2 CC8.1 unverifiable. First thing a
> technical-DD engineer notices.

## Scope

[`.github/workflows/ci.yml`](.github/workflows/ci.yml) line 11:

```yaml
# Auto-trigger disabled while GitHub Actions billing is blocked
# at the account level. Every PR's code is verified locally
# before merge as a matter of policy.
on:
  workflow_dispatch:
```

This brief restores automatic triggering on `push` to main and
on every `pull_request`. The job matrix itself (fmt + clippy +
test) is intact and works when manually fired; only the trigger
needs restoring.

Two paths depending on GitHub Actions billing state at the time
the brief is claimed:

- **Path A (preferred): billing restored.** Replace the
  `workflow_dispatch:` block with the original
  `push: branches: [main]` + `pull_request:` triggers. The
  comment header documenting the disable can be deleted.
- **Path B: billing still blocked.** Wire the same job matrix to
  a self-hosted runner (`runs-on: [self-hosted, linux, x64]`)
  AND keep `workflow_dispatch:` as a manual fallback. Document
  in the workflow header how to provision a self-hosted runner
  (one-time op task — link to GitHub docs, do NOT bundle a
  setup script).

Operator should confirm which path applies before claim. If
unclear, claim and ask in the PR body.

## Files you may touch

- `.github/workflows/ci.yml` — narrowly: replace the `on:` block;
  optionally swap `runs-on:`; remove the disable header
- `.github/workflows/coverage.yml` (NEW, OPTIONAL) — only if
  Path A and you want coverage-as-side-effect; otherwise leave to
  `mission-coverage-tooling`

## Files you must NOT touch

- Any other workflow file (docs.yml, oci-smoke.yml, release.yml are
  separate concerns)
- Any Rust file in this brief (CI re-enable is config-only)

## Acceptance criteria

- [ ] CI workflow triggers on `push` to main
- [ ] CI workflow triggers on every `pull_request`
- [ ] All three jobs (fmt + clippy + test) green on a no-op
      verification PR opened against this brief
- [ ] PR body includes a screenshot or `gh run list` output proving
      green run
- [ ] If self-hosted path: documentation block at the top of
      ci.yml explaining the setup
- [ ] No silent-skip changes to existing tests (silent-skip
      replacement is a separate, larger backlog item)

## Branch + PR

- Branch: `codex-mission-reenable-ci-workflows`
- PR title: `ci: re-enable automatic triggers on push and pull_request (mission-reenable-ci-workflows)`

## Notes for the implementer

- The deep-research analysis flagged "65 files with silent-skip
  env-gated tests". Do NOT try to fix those here — restoring the
  trigger is the immediate-leverage piece. A separate backlog
  item handles the silent-skip pattern.
- After this lands, every other PR going forward gets automated
  regression coverage. This is the foundation for the
  `mission-coverage-tooling` and `mission-headline-reconciliation`
  briefs.
- BLOCK if billing is genuinely uncrackable AND no self-hosted
  runner is available — that's an operator escalation, not a
  worker decision.
