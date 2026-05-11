# Stream E2-e — Hire flow: Step 4 (Introduce) — meet the teammate, hire commit

**Owner**: claude.
**Depends on**: E0 MERGED; E2-a MERGED; E2-b MERGED; E2-c MERGED; E2-d MERGED.
**Authored against**: origin/main `31c36109` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E2-e-hire-flow-step4-introduce`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E2 brief 5 of 9.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Hire commit | POST `/api/v1/workers` returns `workerId`; mock returns a stub id if 404 | Gateway-mandatory hire endpoint with tenant + quota + audit + role-binding; no client commit fallback | `mission-enterprise-hire-commit-mandatory` |
| Persona avatar generation | Initials fallback only (`(M)` for Mariah) | Operator-controlled: initials / generated AI avatar / uploaded image, with safety review on uploads | `mission-enterprise-persona-avatar-options` |
| Example replies trust | Drafted by Genesis once at Step 3; rendered as-is | Server-side moderation + per-vertical safety review before display | `mission-enterprise-genesis-output-moderation` (already named) |
| Hire audit | Single audit row from server (assumed) | Full event chain: input → questions → answers → draft → adjust history → commit, all in one signed audit bundle exportable on demand | `mission-enterprise-hire-audit-bundle` |
| Adjust drawer regeneration cost | Each Adjust click re-runs Genesis (free for now) | Per-tenant cost cap on Adjust iterations; soft-warn at 3, hard-block at 5 | `mission-enterprise-hire-adjust-cost-cap` |

v1 is acceptable because Step 4 is where the user reviews
+ approves their teammate — any wrong-ness is correctable
in-flow before commit. The mock-fallback on hire commit
ONLY runs in development (`NODE_ENV === 'development'`);
production hire commits MUST hit a real gateway or error out
hard.

## Why

Step 4 is the load-bearing emotional moment of the hire
flow ([wireframe-hire-flow.md §"Step 4 — Introduce"](../../skycot/ux-research/wireframes/wireframe-hire-flow.md)).
The user sees their proposed teammate as a profile card,
reads 3 example replies generated in the user's voice, and
either commits ("Hire Mariah") or adjusts ("✎ Adjust
Mariah" — opens a drawer for charter / duty / voice
editing).

This is the moment the abstract "AI teammate" becomes
concrete + emotionally felt. If the example replies don't
sound like the user, the user hits Adjust. If they do, the
user hires + the worker enters their team.

## Scope

- Replace `/hire/step/4` placeholder with the Introduce
  view from the wireframe.
- Render the WorkerDraft from draft state:
  - Avatar (lg, initials by default)
  - Name (editable inline)
  - Role tagline (editable inline)
  - Charter (2-3 sentence block, editable inline)
  - Duties (3-5 rows, each toggleable)
  - 3 example reply previews
- "Adjust" button (top-right of the worker card) opens
  AdjustDrawer.
- AdjustDrawer (Radix Dialog from right):
  - Free-text "What would you change about how
    Mariah works?" textarea.
  - Save → re-runs the Genesis draft mutation (E2-d's
    hook), then re-renders Step 4 with new draft.
  - Cancel → closes drawer, no changes.
- "Hire Mariah" button (primary, lg):
  - Disabled if Adjust drawer is open or in re-draft.
  - On click: POST `/api/v1/workers` with the draft;
    response gives `workerId`.
  - Optimistic transition to Step 5 (Connect) with the
    new workerId attached to draft state.
  - On error: surface a toast + leave the user on
    Step 4 with the original draft.
- Adjust counter: 3+ Adjusts surfaces a fallback
  `"Want to start with a specific template?"` + the 5
  example tiles from E2-b (per wireframe).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots:
  - `web/app/(hire)/hire/step/[step]/page.tsx` (step=4
    case)
  - `web/lib/api/genesis.ts` (re-uses the existing
    `draftWorker` from E2-d)

## Files you may touch

- `web/app/(hire)/hire/step/[step]/page.tsx` — narrowly
  scoped: add `step=4 → Step4Introduce` render path.
  ≤ 300 LOC after.
- `web/components/skycot/hire/Step4Introduce.tsx` — NEW.
  Main step component. ≤ 400 LOC.
- `web/components/skycot/hire/WorkerDraftCard.tsx` — NEW.
  Renders the worker profile card with inline-editable
  fields. ≤ 250 LOC.
- `web/components/skycot/hire/ExampleReplyCard.tsx` —
  NEW. Renders one inbound/outbound example with a
  ChatBubble-like layout. ≤ 150 LOC.
- `web/components/skycot/hire/AdjustDrawer.tsx` — NEW.
  Side drawer (Radix Dialog from right) with free-text
  adjustment input + re-draft trigger. ≤ 220 LOC.
- `web/components/skycot/hire/DutyToggleRow.tsx` — NEW.
  One duty row: label + schedule chip + toggle. ≤ 120
  LOC.
- `web/lib/api/workers.ts` — NEW. `hireWorker(draft):
  Promise<{ workerId: string }>`. Calls gateway via
  the E0 client; dev-only mock fallback returns
  `workerId: "stub-${nanoid()}"`. Production mode
  hard-errors on 404. ≤ 180 LOC.
- `web/lib/hooks/use-hire-worker.ts` — NEW. TanStack
  Query mutation hook wrapping `hireWorker`. Handles
  the route push on success. ≤ 120 LOC.
- `web/components/skycot/hire/Step4Introduce.test.tsx`
  — NEW. ≥ 8 tests: renders draft fields, inline-edit
  name updates state, Adjust opens drawer, save fires
  re-draft, 3+ adjusts surfaces template fallback,
  Hire commit fires mutation, optimistic step
  advancement, error path stays on Step 4. ≤ 350 LOC.

## Files you must NOT touch

- `web/app/(hire)/layout.tsx`, `HireShell.tsx`,
  `ProgressDots.tsx` — locked from E2-a.
- All E2-b through E2-d files — locked from those
  briefs (but `lib/api/genesis.ts:draftWorker` and
  the related hook are reused via import, not edited).
- `web/lib/hire/draft-store.ts`, `use-hire-draft.ts`
  — locked from E2-a.
- Any non-hire surface.
- `crates/**` — sibling backend brief, not in this PR.

## Acceptance criteria

- [ ] `/hire/step/4` renders Step 4 Introduce.
- [ ] If draft state has no `workerDraft`, redirect
      to `/hire/step/3` to trigger drafting.
- [ ] Headline: `"Meet your new teammate."` `text-2xl
      font-semibold`.
- [ ] WorkerDraftCard renders:
      - Avatar (lg, initials from `suggestedName`)
      - Name + role tagline
      - Charter (3 sentence summary)
      - "Adjust" ghost button top-right
- [ ] Inline-edit on name + tagline + charter:
      click-to-edit, blur-to-save, Esc-to-cancel.
- [ ] Duties block: 3-5 rows from
      `workerDraft.duties`, each with a toggle (checkbox)
      that controls `defaultEnabled` in the draft state.
- [ ] 3 example reply cards render below the worker
      card with inbound/outbound message pairs.
- [ ] "Adjust" opens AdjustDrawer from the right.
- [ ] Drawer free-text input + Save fires the
      `draftWorker` mutation with `{ problem, answers,
      adjustment: <text> }` (extends E2-d's
      signature — narrowly add `adjustment?: string`
      param; document the addition).
- [ ] On Adjust success: drawer closes, draft state
      updates, Step 4 re-renders.
- [ ] 3rd Adjust click: surface `"Want to start with a
      specific template?"` + 5 example tiles below
      AdjustDrawer.
- [ ] "Hire Mariah" button (primary, lg) at the foot:
      - Disabled while Adjust drawer open OR re-draft
        in-flight.
      - Loading state during commit.
      - Routes to `/hire/step/5` on success, with
        `workerId` in draft state.
- [ ] Hire commit error: toast surface +
      stay-on-step-4. Strings: title `"Couldn't hire
      ${name}"` + body `${error.message}` + CTA `"Try
      again"`.
- [ ] Dev mode (NODE_ENV='development' AND
      NEXT_PUBLIC_ENV !== 'production'): 404 on
      hire endpoint → fallback to stub workerId.
      Production: 404 → error path.
- [ ] 8 unit tests pass.
- [ ] Playwright e2e: complete Steps 1+2+3+4 via stub
      fallback; assert `/hire/step/5` reached.
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm build` clean. Bundle size delta < 8KB
      First Load JS.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E2-e-hire-flow-step4-introduce`
- PR title: `web(hire): Step 4 (Introduce) — meet teammate + adjust + hire commit (E2-e)`

## Notes for the implementer

- **Worker card layout.** Match the wireframe ASCII
  sketch — avatar+name top-left, tagline below,
  "Adjust" ghost button top-right, charter block
  middle, duties block at the bottom of the card.

- **Inline editing pattern.** For name + tagline:
  - Default render: `<span onClick>` toggles to input.
  - Input is sized to match the span (no layout jump).
  - Blur or Enter saves; Esc cancels.
  - Save updates `useHireDraft().update({
    workerDraft: { ...draft.workerDraft, suggestedName:
    newValue } })`.

- **Charter editor.** Slightly different from name —
  click opens a small inline textarea (3-4 rows max).
  Same save/cancel semantics.

- **Duty toggle.** Simple `<input type="checkbox">` per
  row. Toggle updates the
  `workerDraft.duties[i].defaultEnabled` field.

- **AdjustDrawer.**
  - Mounted via Radix Dialog with side="right".
  - Width: `max-w-md` on desktop; full-screen on mobile.
  - Single textarea + Cancel + Save buttons.
  - On Save: fires `draftWorker(problem, answers,
    adjustment)` mutation; closes drawer on success.
  - Track count of Adjusts in component state. After
    3, render the template-picker fallback inline
    below the drawer's save button (instead of
    forcing re-Genesis).

- **Hire commit.**
  - Endpoint: `POST /api/v1/workers`
  - Request body: `{ workerDraft, problem, answers }`
    — the full hire context. Gateway records the audit
    bundle.
  - Response: `{ workerId, success: true }`.
  - 404 → dev fallback returns
    `{ workerId: "stub-${nanoid()}" }`; production
    raises ApiError.

- **Optimistic step advancement.** On Hire click:
  - Mutation `onMutate`: update draft to
    `{ step: 5, workerId }`. Route to /hire/step/5.
  - Mutation `mutationFn`: actual POST.
  - On success: nothing more (state already advanced).
  - On error: toast + roll back step to 4. The user
    sees Step 4 again with the same draft and the
    error message.

- **Why optimistic advancement?** Because Step 5 is
  Connect (OAuth), which the user wants to start
  immediately. Waiting 1-2s for the hire commit
  before showing Connect feels janky. The trade-off
  is correct here — Connect is read-only browsing of
  available connectors; if the hire commit ultimately
  fails, we roll back gracefully.

- **Error UX.** Toast styled with `tone="error"`,
  positioned `bottom-right` (existing Toast primitive
  from Track C). Auto-dismiss 6s; CTA "Try again"
  re-fires the mutation.

- **Avatar generation v1.** Only initials — derived
  from `workerDraft.suggestedName`. The wireframe
  notes image upload is deferred to v2. Per the
  enterprise hardening backlog, AI-generated avatars
  and uploads are gated by safety review (added in
  `mission-enterprise-persona-avatar-options`).

- **Voice profile editing** is intentionally not
  exposed on Step 4. It's accessible via Adjust
  drawer (free-text "make her more formal" → re-
  Genesis updates the voice profile) and via the
  worker profile after hire. Exposing tone /
  formality sliders at hire would overwhelm.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-hire-commit-mandatory` —
  remove dev-fallback in production builds.
- `mission-enterprise-persona-avatar-options` —
  initials + AI-generated + upload with safety
  review.
- `mission-enterprise-hire-audit-bundle` — full
  hire-flow audit chain in a single signed bundle.
- `mission-enterprise-hire-adjust-cost-cap` — per-
  tenant cost cap on re-Adjust iterations.

Sibling backend brief (prerequisite for v1.5):

- `mission-gateway-workers-hire-endpoint` — POST
  `/api/v1/workers` server-side implementation.
