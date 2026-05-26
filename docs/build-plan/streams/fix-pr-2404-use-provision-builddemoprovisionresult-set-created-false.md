# fix-pr-2404-use-provision-builddemoprovisionresult-set-created-false

**Owner**: claude.
**Authored against**: origin/main `ea161df3a` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-pr-2404-use-provision-builddemoprovisionresult-set-created-false`.
**Source**: evaluator round 85 (S2) `docs/skycot/evals/scheduled/2026-05-26/round-85-pr-2404-onboarding-demo-bypass-post-merge.md`.

🟡 **P2 typecheck regression introduced by PR #2404.** `web/lib/onboarding/use-provision.ts:56-63` `buildDemoProvisionResult()` returns an object that does NOT satisfy the `ProvisionResult` interface — `created: boolean` is required (added by #2376) but the demo builder omits it. `pnpm --dir web exec tsc --noEmit` exits 2 on main with `TS2741 Property 'created' is missing`. Runtime is correct (the `if (res.created)` celebration toast is falsy by default, which is the desired demo behavior), but the build is type-broken.**

## Findings this brief closes

- evaluator round-85 S2 — `lib/onboarding/use-provision.ts(58,3) TS2741` regression on origin/main post-#2404

## Fix recipe

1. **Locate** `web/lib/onboarding/use-provision.ts:56-63`.

2. **Add `created: false`** to the returned object:
   ```ts
   function buildDemoProvisionResult(workspaceName: string): ProvisionResult {
     const slug = slugifyForId(workspaceName);
     return {
       tenantId: `demo-tenant-${slug}`,
       workspaceId: `demo-workspace-${slug}`,
       created: false,
     };
   }
   ```
   Demo mode is by definition an idempotent local replay; `created: false` is the semantically correct value — it suppresses the celebration toast on every demo run, consistent with the comment on `ProvisionResult.created` ("we never falsely celebrate a re-call").

3. **Verify** `pnpm --dir web exec tsc --noEmit` exits 0 (currently exits 2).

4. **Verify** `pnpm --dir web exec vitest run lib/onboarding` still green (no behavioral change expected — the demo path callers don't read `created`).

## Acceptance criteria

- [ ] `buildDemoProvisionResult()` returns an object satisfying `ProvisionResult`
- [ ] `pnpm --dir web exec tsc --noEmit` exits 0
- [ ] `pnpm --dir web exec vitest run lib/onboarding` green
- [ ] Demo-mode workspace step does NOT show the "freshly provisioned!" celebration toast (because `created: false`)

## May-touch

- `web/lib/onboarding/use-provision.ts` (~1 LOC add)

## Out of scope

- Reshaping `ProvisionResult` (interface is canonical per #2376)
- Demo-mode behavior of other onboarding steps

## Effort

~1 LOC. ~5 minutes. Trivial typecheck-regression fix flagged by evaluator round 85.

## Notes

This is the second conductor-rebase TS regression in 48h (evaluator round-85 process note). Evaluator separately recommends a P3 brief `ops-conductor-personal-rebase-runs-typecheck-on-rebased-head` to add a typecheck step to the conductor's personal rebase workflow — handled by a separate brief, not in scope here.
