# fix-onboarding-pick-vertical-demo-mode-honest-install-relabel

**Owner**: claude.
**Authored against**: origin/main `9c407cd20039adf25a986343b7c8cdba76650a52` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-pick-vertical-demo-mode-honest-install-relabel`.
**Source**: UX-AUDIT-T0015 synthesis (`docs/skycot/ux-audits/2026-05-27-0015/onboarding-hire-an-ai-teammate.md` F6 [P1]).

🟠 **P1 — Pick-vertical step subtitle promises "We install the skills, memory schema, and approval gates" but the demo-mode short-circuit at `pick-vertical/page.tsx:226-229` advances straight to `/first-hire` without making any install call. Same `feedback_no_theater_in_demo` violation shape as the provider-key fake validation (covered by `fix-onboarding-demo-mode-fake-validation-and-synthetic-worker-redirect`). The camera sees the buyer click a tile, hear "installing skills, memory schema, approval gates" via the on-screen subtitle, and instantly land on first-hire — nothing was installed.**

## Root cause

`web/app/(setup)/onboarding/pick-vertical/page.tsx:226-229`:

```ts
if (isDemoMode()) {
  setBusy(false);
  router.push("/onboarding/first-hire");
  return;
}
```

Subtitle on the same page (lines 264-266):

> "We install the **skills, memory schema, and approval gates**. You can swap or extend later."

The subtitle is a promise about a function-side outcome. In demo mode the install is not performed — there is no POST to `/api/v1/onboarding/install-vertical` and no gateway round-trip. The skill-count chips on each tile (8 for gtm-engineer, 5 for marketing-intern, etc.) come from the bundled client-side fallback list at `pick-vertical/page.tsx:108-158`. The buyer can verify (e.g. by opening DevTools-network) that no install happened, and a savvy investor will notice the silent advance.

Per `feedback_no_theater_in_demo` (operator-locked 2026-05-24): function-side promises the camera sees must be real, or honestly relabeled.

## Findings this brief closes

- `docs/skycot/ux-audits/2026-05-27-0015/onboarding-hire-an-ai-teammate.md` F6 [P1] — pick-vertical step also skips the install in demo mode (function-side theater on subtitle promise)

## Fix recipe

Two options. Recommend (b) for v1.0 demo timeline; (a) is the right long-term shape and pairs cleanly with the provider-key fix in `fix-onboarding-demo-mode-fake-validation-and-synthetic-worker-redirect`.

**Option A — make demo-mode actually install (preferred for v1.x, paired with provider-key brief)**:

1. Drop the `isDemoMode()` short-circuit at `pick-vertical/page.tsx:226-229`.
2. Let the `POST /api/v1/onboarding/install-vertical` call run in demo mode too. The gateway route at `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs` already accepts a demo-mode header path — re-use the same `X-Argo-Demo-Mode: 1` header pattern proposed in `fix-onboarding-demo-mode-fake-validation-and-synthetic-worker-redirect`.
3. Server responds with the real `{skills_added: N, memory_schema: "...", approval_gates_added: N}` envelope (demo-seed scoped to the demo tenant). The camera sees a real install.
4. SPA renders the existing busy-state spinner during the real round-trip and forwards on success.

**Option B — honest demo-mode relabel (preferred for v1.0 cutover)**:

1. Branch the subtitle string in demo mode. In `web/app/(setup)/onboarding/pick-vertical/page.tsx:264-266`:

   ```tsx
   {isDemoMode()
     ? "Demo mode — pick a starting template. We'll install the real skills, memory schema, and approval gates once you create a real workspace."
     : "We install the skills, memory schema, and approval gates. You can swap or extend later."}
   ```

2. Add a small "Demo mode" pill / chip on each card so the buyer knows the install is deferred.
3. Keep the short-circuit (no install call), but the on-screen promise now matches what happens.

The honest-relabel pattern is already established for `/auth/signup`'s "Demo mode is active — no email is sent" banner — carry it through to every demo-mode short-circuited step.

## Acceptance criteria

- [ ] In demo mode, the pick-vertical subtitle does NOT claim "We install …" as a present-tense action.
- [ ] In demo mode, clicking a tile either (a) fires a real POST and surfaces a real install envelope, OR (b) shows an inline "Demo mode — install deferred" chip.
- [ ] No `console.warn` / no `fetch`-then-discard noise in DevTools-network when in demo mode (Option B). For Option A: a real request appears in network and resolves 200.
- [ ] `pnpm --dir web exec tsc --noEmit` green.
- [ ] `pnpm --dir web exec vitest run app/\(setup\)/onboarding/pick-vertical` passes (add a test that asserts the demo-branch copy ships when `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`).

## May-touch

- `web/app/(setup)/onboarding/pick-vertical/page.tsx` (~30 LOC for Option B; ~60 LOC for Option A)
- `web/lib/skycot/demo-mode.ts` (no change — reuse `isDemoMode()`)
- (Option A only) `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs` (~40 LOC if not already supporting `X-Argo-Demo-Mode: 1` for this endpoint)
- One vitest spec under `web/app/(setup)/onboarding/pick-vertical/__tests__/`

## Out of scope

- The fake-provider-key demo-mode validation theater (covered by `fix-onboarding-demo-mode-fake-validation-and-synthetic-worker-redirect`)
- Welcome-pillar Genesis tile being non-clickable (covered by `fix-onboarding-welcome-step-genesis-studio-framing`)
- Drift between hardcoded skill counts on cards vs gateway truth (covered by `fix-onboarding-pick-vertical-skill-counts-and-gateway-catalog`)
- Replacing pick-vertical with Genesis flow entirely (operator-decisional)

## Effort

S (Option B, ~30 LOC + test) / M (Option A, ~100 LOC across SPA + Rust). Recommend S now; queue M as follow-up.
