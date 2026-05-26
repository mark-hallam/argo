# fix-hire-step-out-of-range-redirect

**Owner**: claude.
**Authored against**: origin/main `6f5d5c49da2e` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-hire-step-out-of-range-redirect`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1441/whole-site-mvp-claims-coverage.md` F22 [P2].

🟡 **P3 — `/hire/step/6` (or any out-of-range step index) returns a generic 404. The hire wizard is a 5-step F1 editorial flow; step 6 doesn't exist. Polish miss — should clamp the URL and redirect to a valid step.**

## Root cause

`web/app/(hire)/hire/step/[step]/page.tsx` parses the `step` param without bounds-checking. The wizard has steps 1..5 inclusive (per the locked F1 design); anything outside that range falls through to Next.js's default 404 handler.

Curl evidence:
```
curl -sI http://localhost:62927/hire/step/6 → HTTP/1.1 404 Not Found
curl -sI http://localhost:62927/hire/step/0 → HTTP/1.1 404 Not Found
curl -sI http://localhost:62927/hire/step/abc → HTTP/1.1 404 Not Found
```

Operator behaviors that trigger this:
- Mistyped URL ("I'll just jump ahead to step 6...")
- Stale browser tab from before a step was removed
- Bookmark saved on a step that no longer exists
- Bot/crawler probing the URL space

## Findings this brief closes

- whole-site-mvp-claims-coverage F22 [P2] — `/hire/step/6` etc 404 instead of redirecting to /hire

## Fix recipe

1. **Add bounds-check in `page.tsx`** at `web/app/(hire)/hire/step/[step]/page.tsx`:
   ```tsx
   import { redirect } from "next/navigation";

   export default function HireStepPage({ params }: { params: { step: string } }) {
     const n = Number.parseInt(params.step, 10);
     if (!Number.isFinite(n) || n < 1 || n > 5) {
       redirect("/hire");  // server-side redirect to step 1
     }
     // existing render logic
   }
   ```

2. **Define the valid step range as a constant** at the top of the file or a shared module:
   ```ts
   export const HIRE_STEPS = ["role", "context", "skills", "approvals", "review"] as const;
   export const HIRE_STEP_COUNT = HIRE_STEPS.length;
   ```
   Use `HIRE_STEP_COUNT` in the bounds check so the redirect adapts if step count changes.

3. **Smoke test** `web/tests/hire-step-bounds.test.ts`:
   - `/hire/step/0` → 307 to `/hire`.
   - `/hire/step/6` → 307 to `/hire`.
   - `/hire/step/abc` → 307 to `/hire`.
   - `/hire/step/1` through `/hire/step/5` → 200.

## Acceptance criteria

- [ ] Out-of-range numeric step → 307 redirect to `/hire`.
- [ ] Non-numeric step → 307 redirect to `/hire`.
- [ ] Valid steps 1..5 still render correctly.
- [ ] Smoke test covers all 4 cases.
- [ ] No regression on the `/hire` (step 1) page.

## May-touch

- `web/app/(hire)/hire/step/[step]/page.tsx` (bounds check)
- `web/app/(hire)/hire/steps.ts` (NEW or extend — HIRE_STEPS constant)
- `web/tests/hire-step-bounds.test.ts` (NEW)

## Out of scope

- The hire wizard step content (already locked)
- `/hire` (no step) routing (separate)

## Effort

~25-40 LOC + 1 test. ~30-45 minutes.
