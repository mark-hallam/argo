# Stream E2-c — Hire flow: Step 2 (Listen) — Genesis-lite clarifying questions

**Owner**: claude.
**Depends on**: E0-frontend-foundation MERGED; E2-a-hire-flow-shell MERGED; E2-b-hire-flow-step1-land MERGED.
**Authored against**: origin/main `2a512348` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E2-c-hire-flow-step2-listen`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E2 brief 3 of 9.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Genesis-lite classification | Server call via `/api/v1/genesis/classify`; client-side stub fallback (3 keyword-matched generic questions) if gateway 404s | Server-side Genesis-lite always present; no client fallback; tenant + cost + audit metadata mandatory | `mission-enterprise-genesis-mandatory` |
| Cost attribution | Best-effort: response carries cost; client doesn't bill yet | Every Genesis call attributed to a tenant + user; charged against workspace budget; surfaced in the hire flow ("This setup used ~$0.02") | `mission-enterprise-genesis-cost-attribution` |
| Question moderation | None — questions render as returned | Server-side content moderation on generated questions (no PII echo, no offensive output) | `mission-enterprise-genesis-output-moderation` |
| Rate limit | None | Per-user rate limit on Genesis calls (prevents hire-flow abuse) | `mission-enterprise-genesis-rate-limit` |
| Failure recovery | Client retries 3× then offers a "skip to template" escape | Server-side fallback to a curated template library when Genesis fails | `mission-enterprise-genesis-template-fallback` |

v1 is acceptable because Genesis is a soft dependency at
Step 2 — even total failure routes the user to a template-
picker fallback, never a dead-end. The Genesis backend
already exists (`mission-genesis-end-to-end-smoke` MERGED);
this brief assumes it'll be HTTP-exposed by a sibling brief
(`mission-gateway-genesis-classify-endpoint`).

## Why

Per [journey-hire-a-teammate.md §Step 2](../../skycot/ux-research/ux-research/journey-hire-a-teammate.md)
and [wireframe-hire-flow.md §"Step 2 — Listen"](../../skycot/ux-research/wireframes/wireframe-hire-flow.md),
after Step 1 the user sees up to 3 clarifying questions
generated from their input. This is where the platform
*listens*, asking just enough to draft a coherent worker
without subjecting the user to a 20-question intake form.

## Scope

- Replace the `/hire/step/2` placeholder with the Listen
  UI per the wireframe.
- A `useGenesisQuestions(problemText)` hook that calls
  the gateway. Returns
  `{ questions: Question[], status: "loading" | "ready"
  | "error", error?: ApiError }`.
- 3 question card slots, rendering whatever the
  classification returns (1–3 questions). Each card has:
  - A heading (the question text from Genesis).
  - An answer surface — type depends on the question's
    `answer_type` enum: `single-select`, `multi-select`,
    `free-text`, `yes-no`.
- Continue button disabled until all questions answered
  (or skippable; defaults: answer required).
- On Continue: save answers to draft state, advance to
  Step 3.
- Skip-to-template fallback when Genesis fails 3×:
  surface a template picker (use the same 5 examples
  from E2-b's `EXAMPLE_ROLES`).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: `web/app/(hire)/hire/step/[step]/page.tsx`
  (locks the step-2 case)

## Files you may touch

- `web/app/(hire)/hire/step/[step]/page.tsx` — narrowly
  scoped: switch on `step` value and render `Step2Listen`
  for step=2. ≤ 200 LOC after changes.
- `web/components/skycot/hire/Step2Listen.tsx` — NEW.
  The Listen step view. Receives draft state, calls the
  Genesis hook, renders question cards. ≤ 350 LOC.
- `web/components/skycot/hire/QuestionCard.tsx` — NEW.
  Renders a single question with the right answer
  surface based on `answer_type`. ≤ 250 LOC.
- `web/lib/api/genesis.ts` — NEW. `classifyRole(problem):
  Promise<{ questions: Question[] }>`. Calls gateway via
  the E0 `apiPost` wrapper; falls back to a small client-
  side stub on 404 / 5xx. ≤ 200 LOC.
- `web/lib/hire/use-genesis-questions.ts` — NEW. TanStack
  Query hook wrapping `classifyRole`. Retries 3× with
  backoff. ≤ 150 LOC.
- `web/lib/hire/genesis-types.ts` — NEW. Type defs:
  `Question`, `QuestionAnswer`, `AnswerType`. ≤ 100 LOC.
- `web/lib/hire/genesis-stub.ts` — NEW. Client-side
  fallback: 3 generic questions when Genesis is
  unavailable. Generic questions are NOT specialized to
  the input (that's the point — Genesis stub is a hard
  fallback, not a feature). ≤ 100 LOC.
- `web/lib/api/genesis.test.ts` — NEW. ≥ 5 unit tests:
  happy path, 404 → stub fallback, 5xx → retry then
  fallback, malformed response → fallback, network
  error → fallback. ≤ 200 LOC.
- `web/components/skycot/hire/Step2Listen.test.tsx` —
  NEW. ≥ 5 tests: loading state, questions render,
  Continue disabled until answered, Continue advances
  + persists answers, skip-to-template surfaces after
  3 retries. ≤ 250 LOC.

## Files you must NOT touch

- `web/app/(hire)/layout.tsx`, `HireShell.tsx`,
  `ProgressDots.tsx` — locked by E2-a.
- `web/app/(hire)/hire/page.tsx`,
  `Step1Land.tsx`, `ExamplesPopover.tsx`,
  `lib/hire/examples.ts` — locked by E2-b.
- `web/lib/hire/draft-store.ts`, `draft-types.ts`,
  `use-hire-draft.ts` — locked by E2-a (extend
  draft-types with `answers` if needed via narrowly-
  scoped append; do NOT restructure).
- Any non-hire surface.
- `crates/**` — Genesis HTTP exposure is a sibling
  backend brief.

## Acceptance criteria

- [ ] `/hire/step/2` renders Step 2 Listen UI when
      draft has a `problem` from Step 1.
- [ ] If draft has no `problem`, redirect to `/hire`
      (user navigated directly without completing
      Step 1).
- [ ] On mount, `useGenesisQuestions(problem)` fires.
      Skeleton loading state shown for up to ~5s.
- [ ] Questions render as 1–3 cards (whatever Genesis
      returns).
- [ ] Each card's answer surface matches `answer_type`:
      - `single-select`: radio group.
      - `multi-select`: checkbox group.
      - `free-text`: textarea (3-row min).
      - `yes-no`: 2-button toggle.
- [ ] Continue button: disabled while any question is
      unanswered, enabled when all are answered.
- [ ] On Continue: draft state updates with
      `{ step: 3, answers: <answers map> }`; route
      navigates to `/hire/step/3`.
- [ ] Back button: returns to `/hire` (Step 1) with
      Step 1's textarea pre-filled from draft.
- [ ] Genesis failure handling:
      - Retry up to 3× with 1s, 2s, 4s backoff.
      - After 3 fails: render a fallback panel:
        `"Couldn't figure out the right questions
        for your setup. Want to start with a similar
        template?"` + the same 5 example tiles from
        E2-b.
      - Tapping an example: routes to `/hire/step/2`
        again with that example's description as a
        new `problem` (re-runs Genesis).
- [ ] Fallback path (no gateway):
      - 3 generic questions surface:
        - `"Should they reply on your behalf, or
          draft replies for you to send?"` (single-
          select: auto / draft / decide-later)
        - `"Should they ever take an action without
          asking you?"` (yes-no)
        - `"What name should they use?"` (free-text)
      - These let the user complete the flow even
        with Genesis down. A small banner above the
        questions: `"Backup questions — your full
        setup will happen on the next screen."`
- [ ] PII not leaked in question rendering: the
      raw user input from Step 1 is NEVER rendered
      back to the user in this step (avoid echo
      attacks).
- [ ] 10 unit tests pass (5 API + 5 component).
- [ ] Playwright e2e: complete Step 1 + Step 2 happy
      path with the stub fallback (so test is
      deterministic without a real gateway).
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm build` clean. Bundle size delta < 6KB
      First Load JS.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E2-c-hire-flow-step2-listen`
- PR title: `web(hire): Step 2 (Listen) — Genesis-lite clarifying questions (E2-c)`

## Notes for the implementer

- **Genesis endpoint shape (assumed):**
  - `POST /api/v1/genesis/classify`
  - Request: `{ problem: string }` (in v1; v1.5 adds
    tenant + user context server-side via the bearer).
  - Response:
    ```json
    {
      "classification": "marketplace-seller",
      "questions": [
        { "id": "q1", "text": "...", "answer_type": "single-select", "options": ["..."] },
        { "id": "q2", "text": "...", "answer_type": "free-text" },
        { "id": "q3", "text": "...", "answer_type": "yes-no" }
      ]
    }
    ```
  - 404 → backend not yet wired → fallback to stub.
  - 5xx → retry → fallback.

- **`Question` type:**
  ```ts
  export type AnswerType = "single-select" | "multi-select" | "free-text" | "yes-no";

  export interface Question {
    id: string;
    text: string;
    answer_type: AnswerType;
    options?: string[];        // required for *-select
    required?: boolean;        // defaults to true
  }

  export type QuestionAnswer =
    | { question_id: string; type: "single-select"; value: string }
    | { question_id: string; type: "multi-select"; values: string[] }
    | { question_id: string; type: "free-text"; value: string }
    | { question_id: string; type: "yes-no"; value: boolean };
  ```

- **Question card layout** per wireframe: numbered
  heading + answer surface, in a Card primitive. 3 cards
  stack vertically with `space-y-3`.

- **Continue validation.** Reduce all answers to
  filled/unfilled:
  - single-select: filled when `value != null`.
  - multi-select: filled when `values.length > 0`.
  - free-text: filled when `value.trim().length > 0`.
  - yes-no: filled when `value !== null`.

- **PII echo protection.** Don't `<RedactedText>` the
  user's Step 1 input here; just don't render it. The
  questions are derived but not literal echoes.

- **Skip-to-template flow.** After 3 Genesis fails, the
  fallback panel replaces the question cards. The 5
  example tiles below the message reuse the
  `EXAMPLE_ROLES` constant from E2-b. Tapping an example
  routes back to `/hire/step/2` with that example's
  description as the new `problem` — the hook fires
  again with the new input.

- **Stub questions are generic by design.** Don't try
  to make them clever or input-aware. They exist solely
  so the user can complete the flow when Genesis is
  unavailable. The architectural target requires real
  Genesis; the stub is a fallback, not a feature.

- **Draft state update.** Append answers to the existing
  draft, keyed by question id:
  ```ts
  updateDraft({ step: 3, answers: { ...draft.answers, ...newAnswers } });
  ```
  Don't replace — Back navigation preserves prior
  answers.

## Hardening briefs implied (Track H)

NOT blocking this PR. Listed for the post-v1 hardening
backlog:

- `mission-enterprise-genesis-mandatory` — server-side
  Genesis always present; no client fallback.
- `mission-enterprise-genesis-cost-attribution` —
  per-tenant cost tracking on Genesis calls.
- `mission-enterprise-genesis-output-moderation` —
  server-side moderation on generated questions.
- `mission-enterprise-genesis-rate-limit` — per-user
  rate limit on hire-flow Genesis calls.
- `mission-enterprise-genesis-template-fallback` —
  server-side curated template library when Genesis
  fails.

Plus one sibling backend brief that's a prerequisite for
v1 to use real Genesis output (not blocking — fallback
covers the gap):

- `mission-gateway-genesis-classify-endpoint` — HTTP
  exposure of the existing Genesis classifier at
  `/api/v1/genesis/classify`.
