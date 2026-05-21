# web-builder-controlled-state-e2e-ci-wire

**Authored against**: `origin/main` `72e782df` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-builder-controlled-state-e2e-ci-wire`.

## Summary

Deferred by #1944. The E2E test `tests/e2e/builder-controlled-state.spec.ts` covers the drag-to-reorder round-trip but is currently NOT wired into the Playwright CI run path (#1944 PR body says "defer to CI (requires dev server)"). Land that wiring.

## Acceptance

- [ ] The Playwright config picks up `tests/e2e/builder-controlled-state.spec.ts` automatically (it's under the configured `testDir`).
- [ ] An npm/pnpm script exists or is documented that runs the spec against a local dev server: `pnpm -C web test:e2e -- builder-controlled-state` or equivalent.
- [ ] CI workflow (if currently enabled — note many are disabled per `.github/workflows/DISABLED_WORKFLOWS_POLICY.md`) runs the spec. If GHA still disabled per operator policy, document the manual run command in the brief PR body and the operator-runnable command in `docs/skycot/operator/`.
- [ ] Spec passes locally against `pnpm -C web dev` (verified by the worker; cite the run output in the PR body).
- [ ] If the spec needs adjustments to run reliably (port assignments, dev-server boot wait, etc.), make them — but do NOT change the assertions.

## May touch

- `web/playwright.config.ts` or equivalent
- `web/tests/e2e/builder-controlled-state.spec.ts` (only for reliability tweaks, not assertion changes)
- `web/package.json` scripts section (additive only)
- `docs/skycot/operator/builder-e2e-run.md` (new file, if operator-runnable docs are the deliverable)
- `.github/workflows/playwright-e2e.yml*` (only if GHA workflows are NOT disabled — check policy first)

## Must NOT touch

- The assertions in the existing spec (they're correct per #1944 PR body).
- `web/lib/hooks/use-workflow-builder-state.ts`.
- Re-enable any `.disabled` GHA workflow — operator-locked per [DISABLED_WORKFLOWS_POLICY.md](.github/workflows/DISABLED_WORKFLOWS_POLICY.md).

## Notes

- The spec is at `tests/e2e/builder-controlled-state.spec.ts` per the #1944 PR body.
- Operator's GHA workflows are intentionally disabled until launch; if this brief can land without re-enabling a workflow, prefer that path.
- Worker can choose: (a) wire into a non-disabled workflow, (b) document the operator-runnable command, (c) both.
