# fix-onboarding-first-hire-no-post-create-verify-probe

**Owner**: claude.
**Authored against**: origin/main `80e149df55f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-first-hire-no-post-create-verify-probe`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1028/onboarding-hire-an-ai-teammate.md` F8 [P1].

🟠 **P1 — `useFirstHire` treats any HTTP 201 with a `worker_id` field as success and immediately pushes to `/workers/{id}`. There's no read-back probe; if the read path 404s (tenant mismatch — see F1), the user lands on a broken page instead of a wizard-side error.**

## Root cause

`web/lib/onboarding/use-first-hire.ts:34-53`:
```ts
const res = await apiPost("/api/v1/onboarding/first-hire", body);
if (res.worker_id) {
  return { workerId: res.worker_id, ... };
}
```
There's no `apiGet(/workers/{id})` after the POST. The wizard's `page.tsx` then calls `router.push(/workers/${workerId})`. If the read path 404s for any reason (tenant mismatch per F1, race condition, gateway lag, future regression), the user crashes into the worker-detail error boundary.

This is a belt-and-braces defence against `fix-onboarding-tenant-resolution-cwd-vs-principal-mismatch` regressing in the future. Even after that brief ships, the verification probe protects the wizard from future tenant-resolution drift cheaply.

## Code evidence

`web/lib/onboarding/use-first-hire.ts:34-53` — single POST, no follow-up GET.
`web/app/(setup)/onboarding/first-hire/page.tsx:51` — `router.push("/workers/" + res.workerId)` fires unconditionally after the hook resolves.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F8 [P1] — `useFirstHire` returns optimistic worker_id even when downstream is broken

## Fix recipe

1. **In `web/lib/onboarding/use-first-hire.ts`**, after the POST succeeds:
   - Probe `apiGet("/api/v1/workers/" + workerId + "/overview")` with a short retry budget (3 attempts, 500ms backoff).
   - If the probe returns 200 within budget → resolve as success.
   - If the probe 404s consistently → reject the mutation with a user-friendly error: `"Hiring succeeded but your teammate isn't visible yet. Try refreshing or contact support."`
2. **In `web/app/(setup)/onboarding/first-hire/page.tsx`**, handle the rejection by rendering a wizard-side error banner (matches the existing wizard error register) instead of routing to a broken detail page.
3. **Surface the failure mode to the gateway audit log** — POST a `wizard.first_hire_verify_failed` event with `worker_id` + `tenant_id` so the operator can see if the tenant-mismatch regression returns.
4. **Web test**: mock the gateway to return 201 on first-hire but 404 on the verify probe, assert the wizard renders the error banner and does NOT push to `/workers/<id>`.

## Acceptance criteria

- [ ] `useFirstHire` verifies worker readability before resolving.
- [ ] On verify-failure, the wizard shows a clear error and stays on step 5; does not crash into `/workers/<id>` error boundary.
- [ ] On verify-success, behaviour is unchanged.
- [ ] A `wizard.first_hire_verify_failed` audit event is recorded on failure.
- [ ] Web test covers the failure path.

## May-touch

- `web/lib/onboarding/use-first-hire.ts` (add verify probe)
- `web/app/(setup)/onboarding/first-hire/page.tsx` (handle rejection)
- `web/components/skycot/onboarding/FirstHireErrorBanner.tsx` (NEW small component)
- `web/tests/onboarding-first-hire-verify.test.ts` (NEW)

## Out of scope

- The underlying tenant-resolution fix (separate brief: `fix-onboarding-tenant-resolution-cwd-vs-principal-mismatch`)
- Audit-event schema bump for the new event kind (only if it's not already a generic SPA-side event)

## Effort

~70-120 LOC + 1 test. ~2 hours.
