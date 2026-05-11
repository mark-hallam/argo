# Stream E2-d — Hire flow: Step 3 (Drafting transition) — Genesis-lite worker-draft generation

**Owner**: claude.
**Depends on**: E0-frontend-foundation MERGED; E2-a MERGED; E2-b MERGED; E2-c MERGED.
**Authored against**: origin/main `1f8c8878` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E2-d-hire-flow-step3-drafting`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E2 brief 4 of 9.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Worker draft generation | Server POST to `/api/v1/genesis/draft-worker`; stub fallback if 404 (returns a hardcoded marketplace-seller-shaped draft) | Mandatory server-side Genesis with per-call cost tracking; no fallback | `mission-enterprise-genesis-draft-mandatory` |
| Draft persistence | Client-side in localStorage via E2-a's draft store | Server-side draft with tenant + user + audit row | `mission-enterprise-hire-draft-server-store` (already named in E2-a) |
| Long-running call UX | 5-stage cosmetic progress checklist running on a timer regardless of real backend progress | Server emits real progress events via SSE/WS; UI reflects actual stages | `mission-enterprise-genesis-progress-stream` |
| Failure recovery | After 30s: surface "still working / try again" with retry. After 3 retries: skip-to-template (reuses E2-c fallback path). | Server-side timeout + automatic template fallback; UI never hits the 30s wall | same as above |
| Content moderation | None | Server-side moderation on generated persona, charter, duties (PII scrub, profanity check, brand-safety) | `mission-enterprise-genesis-output-moderation` (also named in E2-c) |
| Voice profile inheritance | v1 uses a default voice profile (warm/casual/standard) | Server reads operator's prior messages from connected channels and infers a starting voice profile | `mission-enterprise-genesis-voice-inference` |

v1 is acceptable because the worker draft is editable in
Step 4 — even a poor draft can be adjusted before commit.
The cosmetic progress timing is a known UX-vs-honesty
tradeoff (called out in the wireframe).

## Why

Step 3 is the cinematic moment of the hire flow. The user
just answered 3 clarifying questions; now the platform
"thinks" for a few seconds and emerges with a full worker
draft (persona, charter, duties, sample replies). The
wireframe ([wireframe-hire-flow.md §"Step 3 — Drafting"](../../skycot/ux-research/wireframes/wireframe-hire-flow.md))
calls for a 3-5 second cosmetic progress checklist while
Genesis-lite generates the draft.

This step is NOT just a wait screen — it triggers the real
heavy-lift Genesis call that produces the data for Step 4.
The user perception of progress + actual server progress
must converge by the time the draft is ready.

## Scope

- Replace `/hire/step/3` placeholder with the Drafting
  transition view.
- Trigger the worker-draft generation on mount via
  `useGenesisWorkerDraft()` (new hook).
- 5-stage cosmetic progress checklist (per wireframe):
  - "Building her persona"
  - "Drafting her first-week routine"
  - "Picking a few opening examples"
  - "Tuning her voice to match yours"
  - "Almost done…"
- Each stage fades-in + check-marks at ~700ms intervals
  regardless of backend progress (the cosmetic timer).
- When backend completes: if all 5 stages have rendered,
  advance to Step 4. If backend is slower, hold at "Almost
  done…" with a subtle spinner.
- 8-second mark: show "Still working…" hint.
- 30-second mark: surface the retry / skip-to-template
  fallback (same UI as E2-c).
- No back/continue buttons in this step — auto-advances.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: `web/app/(hire)/hire/step/[step]/page.tsx`
  (step=3 case)

## Files you may touch

- `web/app/(hire)/hire/step/[step]/page.tsx` — narrowly
  scoped: add `step=3 → Step3Drafting` render path.
  ≤ 250 LOC after.
- `web/components/skycot/hire/Step3Drafting.tsx` — NEW.
  The transition view + progress checklist + retry
  fallback. ≤ 350 LOC.
- `web/components/skycot/hire/DraftingProgressList.tsx`
  — NEW. The 5-stage check-marking list. Pure visual;
  receives `currentStage: number` from parent. ≤ 180 LOC.
- `web/lib/api/genesis.ts` — narrowly scoped: ADD
  `draftWorker(problem, answers): Promise<WorkerDraft>`.
  Existing `classifyRole` from E2-c untouched. ≤ 300 LOC
  after.
- `web/lib/hire/use-genesis-worker-draft.ts` — NEW.
  TanStack Query mutation hook that triggers
  `draftWorker` and writes the result to draft state on
  success. ≤ 180 LOC.
- `web/lib/hire/genesis-stub.ts` — narrowly scoped: ADD
  a `stubWorkerDraft(problem)` function that returns a
  fixed marketplace-seller-shaped draft for fallback.
  ≤ 200 LOC after.
- `web/lib/hire/draft-types.ts` — narrowly scoped: ADD
  `WorkerDraft` type (persona, charter, duties[],
  exampleReplies[]). Keep all prior types intact.
  ≤ 150 LOC after.
- `web/components/skycot/hire/Step3Drafting.test.tsx` —
  NEW. ≥ 6 tests: stages reveal on timer, backend ready
  before timer → still waits for timer, backend slow →
  hold at "Almost done", 30s timeout → retry surfaces,
  retry succeeds → resumes. ≤ 250 LOC.

## Files you must NOT touch

- `web/app/(hire)/layout.tsx`, `HireShell.tsx`,
  `ProgressDots.tsx` — locked from E2-a.
- `web/app/(hire)/hire/page.tsx`, `Step1Land.tsx`,
  `ExamplesPopover.tsx`, `lib/hire/examples.ts` —
  locked from E2-b.
- `web/components/skycot/hire/Step2Listen.tsx`,
  `QuestionCard.tsx`, `lib/hire/use-genesis-
  questions.ts` — locked from E2-c.
- `web/lib/hire/draft-store.ts`, `use-hire-draft.ts`
  — locked from E2-a (extend types only).
- Any non-hire surface.
- `crates/**` — sibling backend brief, not in this PR.

## Acceptance criteria

- [ ] `/hire/step/3` renders the Drafting transition.
- [ ] If draft state has no answers (user navigated
      directly), redirect to `/hire/step/2`.
- [ ] On mount, fires `draftWorker(problem, answers)`
      mutation.
- [ ] 5-stage checklist renders with stage-by-stage
      reveal at ~700ms intervals.
- [ ] Each stage shows a spinner during its active
      moment, then a checkmark when "complete" (cosmetic
      timer, not backend progress).
- [ ] If backend resolves before all 5 stages have
      visually shown: UI waits for the cosmetic timer
      to finish, then advances.
- [ ] If backend is slower than 5 stages: UI holds at
      "Almost done…" with a small spinner.
- [ ] 8-second mark: render `"Still working — usually
      only takes a few seconds."` below the checklist
      (subtle muted text).
- [ ] 30-second mark: replace the checklist with a
      retry panel:
      `"This is taking longer than expected."`
      `[ Try again ]   [ Pick a template instead ]`
- [ ] "Try again": re-fires the mutation; stages reset;
      cosmetic timer restarts.
- [ ] After 3 retries: replace retry panel with the
      skip-to-template fallback (5 example tiles from
      `EXAMPLE_ROLES`).
- [ ] On success: write `workerDraft` to draft state +
      advance to `/hire/step/4`.
- [ ] No Back / Continue buttons rendered (HireShell's
      footer hidden for this step).
- [ ] 6 unit tests pass.
- [ ] Playwright e2e: complete Steps 1+2+3 with stub
      fallback; assert /hire/step/4 reached within 6s.
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm build` clean. Bundle size delta < 6KB First
      Load JS.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E2-d-hire-flow-step3-drafting`
- PR title: `web(hire): Step 3 (Drafting transition) — worker draft generation (E2-d)`

## Notes for the implementer

- **`WorkerDraft` type** (locked):
  ```ts
  export interface WorkerDraft {
    suggestedName: string;
    persona: {
      tagline: string;
      avatarKind: "initials" | "generated" | "uploaded";
      voiceProfile: { tone: "warm" | "neutral" | "formal";
                       formality: "low" | "medium" | "high";
                       signaturePhrases: string[] };
    };
    charter: string;  // 2-3 sentences
    duties: Array<{
      label: string;
      schedule: { kind: "cron" | "event"; spec: string };
      defaultEnabled: boolean;
    }>;
    exampleReplies: Array<{
      inboundContext: string;
      mariahResponse: string;  // (generic name — applies regardless of suggestedName)
    }>;
    approvalPolicy: {
      level: "conservative" | "balanced" | "aggressive";
    };
    suggestedVertical: string;
  }
  ```

- **Cosmetic timer.** A single `useEffect` runs 5
  stages × 700ms = ~3.5s. The timer should NOT be tied
  to backend progress — it runs regardless. When both
  the backend is done AND the timer has hit stage 5,
  advance. If the timer finishes first, hold at "Almost
  done…" until backend returns. If backend finishes
  first, let the timer complete (creates the
  satisfying "thinking" perception).

- **Endpoint shape (assumed):**
  - `POST /api/v1/genesis/draft-worker`
  - Request: `{ problem, answers }` (the Step 1 input
    + Step 2 answers).
  - Response: a `WorkerDraft` per the type above.
  - 404 → fallback to `stubWorkerDraft(problem)`.

- **`stubWorkerDraft` shape.** A fixed marketplace-
  seller-shaped draft. Keep it generic enough that it
  works for any input (the user adjusts in Step 4
  anyway). Hardcoded values:
  - `suggestedName`: "Mariah"
  - `persona.tagline`: "Marketplace assistant"
  - `charter`: "<NAME> helps you manage your inbound
    customer messages. They draft replies in your
    voice and ask you before taking unusual actions."
  - 3 duties: morning summary, draft replies, evening
    check-in.
  - 3 example replies covering common scenarios.

- **Mutation retry strategy.** TanStack Query's
  `useMutation` doesn't retry by default. Wrap with a
  retry count tracker:
  ```ts
  const [retryCount, setRetryCount] = useState(0);
  const mutation = useMutation({ mutationFn: draftWorker });
  // After 30s timeout → setRetryCount(c => c + 1) →
  //   mutation.mutate() again.
  // When retryCount > 3 → render the skip-to-template
  //   fallback instead of retry button.
  ```

- **Hide HireShell footer for Step 3.** HireShell from
  E2-a renders Back + Continue. For this step, those
  buttons must not appear (the step is non-interactive
  except for cosmetic retry/skip). Add an opt-out prop
  to HireShell:
  ```ts
  <HireShell hideFooter={true}>{step3Content}</HireShell>
  ```
  This is a narrow extension to HireShell's interface.
  Document the prop in the existing HireShell file
  rather than introducing a wrapper.

- **The "Almost done…" hold state.** Render a small
  Loader2 icon next to the text. The user perception
  is "the system is still working hard", not "this is
  stuck". Subtle is key.

- **Avoid LLM cost waste.** If user navigates away mid-
  draft (escape, back button on HireShell, or close),
  cancel the mutation:
  ```ts
  useEffect(() => () => mutation.reset(), []);
  ```
  This doesn't cancel the server-side work (no
  cancellation API in v1) but it prevents the client
  from acting on a stale response.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-genesis-draft-mandatory` —
  server-side Genesis worker-draft always present.
- `mission-enterprise-genesis-progress-stream` —
  real SSE/WS progress events from the gateway.
- `mission-enterprise-genesis-voice-inference` —
  server-side voice-profile inference from operator's
  prior messages.

Plus the existing sibling brief:

- `mission-gateway-genesis-classify-endpoint` (now
  needs a `/draft-worker` companion route).

Plus the previously-named:

- `mission-enterprise-genesis-output-moderation` —
  applies to draft output too.
- `mission-enterprise-hire-draft-server-store` —
  draft persistence migrates to server here.
