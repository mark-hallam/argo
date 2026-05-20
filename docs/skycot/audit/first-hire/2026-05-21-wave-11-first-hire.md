# Wave-11 audit #5 — `/onboarding/first-hire` first-emotional-moment audit

**Auditor**: claude (Wave-11 audit agent #5)
**Date**: 2026-05-21
**Audited against**: `origin/main` (page commit `873242b2`, redirect-fix commit `4f4059d9`)
**Mode**: read-only
**Source files**:
- `web/app/(setup)/onboarding/first-hire/page.tsx` (131 LOC)
- `web/lib/onboarding/use-first-hire.ts` (59 LOC)
- `web/lib/onboarding/use-onboarding-draft.ts`
- `web/lib/api/client.ts`
- `web/app/(setup)/onboarding/_OnboardingShell.tsx`
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs` (POST router)
- `web/__tests__/onboarding/wizard.test.tsx`
- `web/tests/e2e/onboarding.spec.ts`
- `web/tests/e2e/deployed/hire-flow-real.spec.ts`

---

## End-to-end score: 4/10

The wave-5 redirect fix landed (one-line; merged in `4f4059d9`), so the success-path 404 is gone — but the success path itself is unreachable because **the gateway endpoint that the page POSTs to does not exist**. The "first emotional moment" today is a red error message that the page itself contradicts (says "we'll create your worker in the background" but no background job is queued anywhere). The page is well-built for what the page can control; the backend gap and the misleading error copy are the demo-blockers.

Breakdown by score axis (out of 10):
- Visual / brand fit: 9 — `OnboardingShell` (post-#1898) puts this page inside `WizardEditorialShell`, so Audiowide hero + purple→pink gradient + warm radial-blob bg + trust strip footer are all present.
- Input UX: 7 — multi-line textarea, helpful placeholder + example.
- Validation: 8 — empty-prompt + missing-vertical guards both fire; copy is plain English.
- Loading state: 8 — `loading={submitting}` on `Button` renders a `Loader2` spinner + `aria-busy`, disables the button.
- Success redirect: 10 — `/workers/<id>` is the canonical detail route (wave-5 win).
- Error states: 3 — error copy is actively misleading (says "we'll create in the background" when nothing is queued).
- Mid-genesis affordance / progressive disclosure: 1 — none. Just a spinner.
- Backend reality: 0 — endpoint does not exist. See finding A.

---

## Dimension-by-dimension findings

### 1. Description input — placeholder + textarea (score: 7/10)

- **Multi-line textarea**: yes (`<textarea className="min-h-32 …">` at `page.tsx:88-95`).
- **Placeholder helpful**: yes — `"A marketing intern named Maya who drafts blog posts and runs weekly competitive scans."` (the demo persona). The same string is also rendered below the textarea as `Example: <em>…</em>` (`page.tsx:97`). Two-place duplication is fine — the placeholder vanishes on focus, the helper text persists.
- **No max-length**: the textarea has no `maxLength`. A user pasting 50KB will trip a gateway request-body limit silently (the gateway returns a JSON 4xx for oversize bodies but the page would render that as the same misleading error).
- **No character count**: the genesis classifier behind this endpoint will be more accurate for prompts in the 50-300 char range; no soft hint nudges the operator there.
- **Auto-focus**: not set — the operator has to click into the textarea after landing.
- **No suggested-prompt chips**: compare to `/skills` and `/hire` which both render "try one of these" suggestion pills. Onboarding's first hire is the operator's most-loaded moment and ships with one bare placeholder.

### 2. "Hire them" button (score: 8/10)

- **Disabled until non-empty**: yes — `canSubmit = prompt.trim().length > 0 && !submitting` (`page.tsx:63`), wired through `<Button disabled={!canSubmit} loading={submitting}>` (`page.tsx:117-126`).
- **Loading state**: yes — `Button` renders `<Loader2 className="animate-spin" aria-hidden />` and sets `aria-busy={loading}` (`Button.tsx:60-64`). Re-clicking is prevented because `disabled || loading` collapses to `true`.
- **No double-click guard beyond the disabled flag**: fine — the `useMutation` itself is idempotent under React Query (request is in-flight, repeated calls become no-ops).
- **Icon + label**: `<Sparkles aria-hidden /> Hire them` — on-brand. (One of three SKYCOT viral moments where Sparkles fires; this is the deepest in the funnel.)

### 3. Genesis call — `/api/v1/onboarding/first-hire` status (score: 0/10) [SHIP-BLOCKER]

**Finding A — endpoint does not exist on the gateway.**

- `grep -rn "first-hire\|first_hire" crates/` returns zero matches. The only `/api/v1/onboarding/*` route the gateway dispatches is `provision` (`crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs:206-210`).
- `routes.rs:177` returns `("404 Not Found", "{\"error\":\"not found\"}")` for any unmapped POST. The web client's `parseError` (`client.ts:45-63`) cannot match the gateway's bare-string `error` field against the `{error:{code,message}}` envelope it expects, so it falls through to `new ApiError(404, "Not Found", undefined, '{"error":"not found"}')`. That ApiError IS thrown and caught — so this is **not** a silent-success bug.
- **Wave-3's "silent-success" bug claim was always inaccurate for this page.** The wave-3 audit (`docs/skycot/audit/onboarding/2026-05-20-wave-3-onboarding-e2e.md:106`) flagged "the first-hire mutation will hard-fail on the live gateway and the user is stuck on step 5 with no Skip button" — which IS what happens. The silent-success bug belonged to the sibling `/onboarding/pick-vertical/page.tsx:74` (its hand-rolled `fetch` swallowed `!res.ok && res.status !== 404` as success). On `first-hire` the error surfaces; it just surfaces wrong (see finding D).
- **Brief is CLAIMED but not MERGED**: `web-gateway-endpoints-onboarding-batch` (claude, CLAIMED 2026-05-20 16:51 UTC per `BUILD_STATUS.md:919`) is the brief authored to ship all three missing endpoints (`provider-key`, `install-vertical`, `first-hire`). Until it merges, the demo first-hire path is held together by Playwright `page.route()` interceptors (`web/tests/e2e/onboarding.spec.ts:157-163`). The deployed-real lane (`web/tests/e2e/deployed/hire-flow-real.spec.ts`) sidesteps the onboarding first-hire endpoint entirely and instead uses `/hire`'s genesis classifier + `/api/v1/workers` POST — a separate, working pipeline.

### 4. Success → `/workers/<id>` redirect (score: 10/10)

- Verified the wave-5 fix landed (commit `4f4059d9`, "fix(web): Wave-3 audit fix-cycle"). `page.tsx:52` now reads `router.push(\`/workers/${res.workerId}\`)` with a 4-line code comment explaining why `/team/` was wrong. The canonical worker-profile route is `web/app/(app)/workers/[workerId]/page.tsx`; the redirect lands there.
- **But the unit test was not updated.** `web/__tests__/onboarding/wizard.test.tsx:197` still asserts `expect(pushMock).toHaveBeenCalledWith("/team/wkr_maya_1")` — when the production code pushes `/workers/wkr_maya_1`. This test is currently **broken vs. production**; either the test passes because vitest's CI gate isn't running this file, or the regression has not been re-run since wave-5 fix. Worth a one-line follow-up.
- The deployed Playwright spec at `web/tests/e2e/onboarding.spec.ts:106` accepts either shape (`\/(team|workers)\/[A-Za-z0-9\-_]+/`) — defensive but stale; safe to tighten to `/workers/` only post-merge.

### 5. Error states (score: 3/10)

**Finding D — error copy contradicts itself + reality.**

`page.tsx:53-59`:

```ts
} catch (e) {
  setError(
    e instanceof Error
      ? `${e.message}. We'll create your worker in the background — try again or skip to your team.`
      : "Genesis call failed. Try again.",
  );
}
```

Problems:

1. **No background job is created.** Nothing posts to a queue; no row hits `workers` with status `hiring`; no replay-friendly side effect remains. The copy is a flat-out lie. On a 404 (which IS what happens today) the operator sees:
   `Not Found. We'll create your worker in the background — try again or skip to your team.`
   That sentence is contradictory ("not found" + "we'll create in background") AND wrong (no background creation happens).
2. **"Skip to your team" suggests an affordance that doesn't exist on this page.** There's no Skip button, no `/team` link, no fallback button next to the error. The only escape is `Back` (which goes to `/onboarding/pick-vertical`, not `/team`).
3. **No retry button.** "Try again" is words on screen with no button — the user has to click "Hire them" again, which re-runs the same broken endpoint and produces the same error.
4. **Differentiation between 4xx-bad-request and 5xx-gateway-down**: zero. A genuine validation error (e.g. description too short) and a connectivity error look identical.
5. **No telemetry.** No `console.error`, no `trackError(...)`, no `Sentry.captureException`. A user who churns out of step 5 leaves no trace.

The error path is the **highest-traffic outcome on the live gateway today** (because the endpoint 404s) and it gets the least design love.

### 6. Mid-genesis affordance / progressive disclosure (score: 1/10)

**Finding E — there is no mid-flight UX. Just a spinner.**

When the operator clicks Hire them, the only visual change is `<Button>` swapping its label icon for a `Loader2` spinning circle. The textarea stays editable (no disabled state), no "Drafting personality..." / "Picking skills..." / "Connecting to Slack..." sequence — even though the brief (`streams/claude-onboarding-wizard.md:20`) explicitly framed this as the viral moment ("Maya is hired! Take you to her profile.").

Compare to `/hire/step/3` (`Step3Drafting`) which IS the viral mid-genesis surface for the team-empty-state path. That step shows:
- A 3-line progressive log ("Reading your problem…" → "Drafting Maya's charter…" → "Picking 4 of 17 skills…")
- A subtle animated gradient bar
- Auto-advances to step 4 on completion

`/onboarding/first-hire` skips that pattern entirely. Given the operator's stated brand-promise ("UX must be epic, must go viral"), this is the single highest-ROI fix on the page if/when the backend lands.

Three viable design moves once the endpoint exists:

- **Cheap**: 3-step progressive copy under the spinner ("Reading your description… Drafting personality… Hiring…"). 30 minutes of work, no backend change.
- **Medium**: redirect immediately to `/workers/<id>?status=hiring` and let the worker profile render a `hiring` placeholder. The `FirstHireResult.pending: boolean` field already exists in the response shape (`use-first-hire.ts:14-20`) for exactly this. No work has wired that field through.
- **Expensive**: port `Step3Drafting` from `/hire/step/3` into `/onboarding/first-hire`. Highest viral impact; also closes the wave-3 recommendation that "the two wizards should look the same".

---

## Off-page tail (worth flagging)

- **`web/__tests__/onboarding/wizard.test.tsx:197`** asserts the dead `/team/<id>` route. Updating to `/workers/<id>` is a 1-character change.
- **Wave-3 audit's "silent-success" claim was about a sibling page**, not `/onboarding/first-hire`. The actual first-hire failure mode today is "user-visible 404 with misleading error copy", not "silent success". Worth correcting in the wave-3 audit's followups if anyone re-reads it.
- **Onboarding draft has no TTL** — if a user lands on first-hire, fails, closes the tab, and returns a week later, `firstHirePrompt` and `verticalId` are still in `localStorage`. Not a bug today but the wave-3 audit flagged this for both wizards (`useHireDraft` has 7-day TTL; `useOnboardingDraft` has none).

---

## Verdict — does the first emotional moment land?

**No, not today.** The page itself is brand-correct (post-#1898 it lives inside `WizardEditorialShell` with Audiowide + gradient + radial-blob), and the wave-5 redirect fix means the *intended* success path is sound. But the actual call hits a non-existent gateway endpoint, the error copy is a contradictory lie, and the mid-genesis "watch your AI hire come to life" moment that the brief promised is just a generic loading spinner. A new user hitting `/onboarding/first-hire` on the live gateway today sees "Not Found. We'll create your worker in the background — try again or skip to your team." and has no Skip button. That is the polar opposite of a viral moment.

**Two unblocks in order:**

1. **Land `web-gateway-endpoints-onboarding-batch` (currently CLAIMED).** Restores the success path.
2. **Port the `Step3Drafting` progressive-disclosure pattern from `/hire/step/3` into the loading state.** Without this, even a working backend produces a one-spinner moment instead of the demo-recorded "Maya is hired!" beat.

The dead `/team/` route bug is closed (wave-5 win). The two open issues are: backend gap (P0) and missing mid-genesis affordance (P1, viral-readiness).
