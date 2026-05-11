# Stream E2-b — Hire flow: Step 1 (Land) — free-text input, examples popover, hire-session start

**Owner**: claude.
**Depends on**: E0-frontend-foundation MERGED; E2-a-hire-flow-shell MERGED.
**Authored against**: origin/main `e5272815` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E2-b-hire-flow-step1-land`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E2 brief 2 of 9.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Hire session start | Local-only — draft state persists, no server call yet | Gateway POST creates a server-side hire session with tenant scope, audit row, quota check | `mission-enterprise-hire-session-server-start` |
| Input validation | Min-15-char client check | Server-enforced shape + content moderation + PII scrub on the input | `mission-enterprise-hire-input-validation` |
| Rate limiting | None | Per-user + per-tenant rate limit on hire attempts (prevents abuse) | `mission-enterprise-hire-rate-limit` |
| Example prompts | Hardcoded 5 examples in client | Operator-configurable example set per workspace (e.g. enterprise customer's curated examples) | `mission-enterprise-hire-examples-configurable` |
| Input audit | Not audited in v1 | Every Step 1 input audit-logged (input text, timestamp, user, tenant) for compliance | `mission-enterprise-hire-input-audit` |

v1 is safe because Step 1 input is just free-text in the
user's own draft — nothing leaves the device until later
steps. Server-side hire-session creation moves to v1.5
when Genesis-lite is wired in E2-c/E2-d and the draft must
sync.

## Why

The hire-flow Step 1 is the moment a user says "I need help
with X" in plain English. It's a single textarea + a
rotating placeholder + an examples popover + a submit button.

The shell (E2-a) renders a placeholder for this step; this
brief replaces the placeholder with the real interaction.

## Scope

- Replace `web/app/(hire)/hire/page.tsx`'s Step 1 placeholder
  with the real Land UI per
  [wireframe-hire-flow.md §"Step 1 — Land"](../../skycot/ux-research/wireframes/wireframe-hire-flow.md).
- Free-text `<textarea>` with auto-grow.
- 3 placeholder strings rotating randomly on mount.
- "Need some examples?" ghost button → popover with 5
  example role-descriptions; tapping fills the textarea.
- Continue button disabled until textarea has ≥ 15 chars.
- Submit on `Cmd/Ctrl + Enter`.
- On submit: write to draft state (E2-a's draft store)
  + advance to Step 2.
- Validation feedback below the textarea when user
  tries to advance with < 15 chars: `"Tell me a bit more —
  even one sentence is enough."`

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: `web/app/(hire)/hire/page.tsx` (locks
  until merge)

## Files you may touch

- `web/app/(hire)/hire/page.tsx` — REPLACE the E2-a
  placeholder with the real Step 1 implementation. ≤
  350 LOC.
- `web/components/skycot/hire/Step1Land.tsx` — NEW. The
  Step 1 view as a self-contained component. Receives
  `{ draft, onAdvance }` from the page. ≤ 250 LOC.
- `web/components/skycot/hire/ExamplesPopover.tsx` — NEW.
  Popover with 5 example role-descriptions. ≤ 180 LOC.
- `web/lib/hire/examples.ts` — NEW. The 5 default
  examples (hardcoded for v1). Exported as a typed
  array `EXAMPLE_ROLES: Array<{ title: string,
  description: string }>`. ≤ 80 LOC.
- `web/lib/hire/use-step1.ts` — NEW. Step 1 form state
  hook (textarea value, validation, submission).
  ≤ 120 LOC.
- `web/components/skycot/hire/Step1Land.test.tsx` —
  NEW. ≥ 6 tests: renders rotating placeholders,
  examples popover opens + fills textarea, Continue
  disabled under 15 chars, Continue enabled at ≥ 15,
  Cmd+Enter submits, validation message shows on
  premature submit. ≤ 250 LOC.

## Files you must NOT touch

- `web/app/(hire)/layout.tsx` — locked by E2-a.
- `web/components/skycot/HireShell.tsx` — locked by E2-a.
- `web/components/ui/ProgressDots.tsx` — locked by E2-a.
- `web/lib/hire/draft-store.ts` / `draft-types.ts` /
  `use-hire-draft.ts` — locked by E2-a.
- Any non-hire surface.
- `crates/**` — backend hire-session brief is sibling,
  not in this PR.

## Acceptance criteria

- [ ] `/hire` renders Step 1 Land UI in place of the
      placeholder.
- [ ] Headline: `"What do you need help with?"`
      `text-3xl font-semibold tracking-tight`.
- [ ] Subtitle: `"Tell me in your own words. Anything
      you'd love to stop doing yourself."`
- [ ] Textarea with `min-h-[160px]`, auto-grows to
      content (max `max-h-[400px]`).
- [ ] 3 rotating placeholders (per wireframe), one
      picked at random on mount, NOT changing during
      a single session.
- [ ] "Need some examples?" ghost button below textarea
      opens an examples popover.
- [ ] Popover shows 5 example rows: marketplace seller,
      recruiter, customer support, sales SDR, tax-prep
      intake (see `lib/hire/examples.ts` for canonical
      copy).
- [ ] Tapping an example fills the textarea + closes
      the popover.
- [ ] Continue button: disabled while textarea < 15
      chars, enabled at ≥ 15. Tooltip on disabled:
      `"Answer the question to continue"` (matches
      wireframe).
- [ ] Cmd/Ctrl + Enter: submits (advances to Step 2).
- [ ] On submit: draft state updates
      `{ step: 2, problem: <textarea value> }` and
      route navigates to `/hire/step/2`.
- [ ] Validation message below textarea when user tries
      to advance with < 15 chars (e.g. clicks the
      disabled Continue or Cmd+Enter): `"Tell me a bit
      more — even one sentence is enough."`. Auto-
      dismisses 5s later or when user types.
- [ ] Back button on Step 1 stays disabled (no
      previous step).
- [ ] 6 unit tests pass.
- [ ] Playwright e2e (extends E2-a's spec): type 20
      chars, hit Continue, assert /hire/step/2 loaded
      with placeholder.
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm build` clean. Bundle size delta < 4KB First
      Load JS.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E2-b-hire-flow-step1-land`
- PR title: `web(hire): Step 1 (Land) — free-text input + examples (E2-b)`

## Notes for the implementer

- **Example role list** (locked in `lib/hire/examples.ts`):
  ```ts
  export const EXAMPLE_ROLES = [
    {
      title: "Marketplace seller",
      description: "I sell hockey gear on Facebook Marketplace and Gumtree. I get 30 messages a day, most are 'is this still available'. I'd love someone to handle that.",
    },
    {
      title: "Recruiter — candidate screening",
      description: "I get 50+ applications a week for engineering roles. I need someone to do first-pass screening, schedule interviews, and keep candidates warm.",
    },
    {
      title: "Customer support — Tier 1",
      description: "We get a flood of tier-1 SaaS support tickets. Most are answerable from our docs. I want someone to handle those and escalate the rest.",
    },
    {
      title: "Sales SDR — outbound",
      description: "I research prospects and send personalized outbound emails. I'd love help with the research + drafting so I can focus on the calls that close.",
    },
    {
      title: "Tax-prep document intake",
      description: "Clients send me piles of W-2s, 1099s, and receipts. I need someone to extract the line items, flag missing docs, and queue them for my review.",
    },
  ];
  ```

- **Placeholder rotation.** Pick on mount using
  `useState(() => EXAMPLES[Math.floor(Math.random() *
  EXAMPLES.length)])` so it doesn't change mid-session.
  Don't change on each render — disorienting.

- **Validation copy** lives next to the Continue button:
  ```
  <button disabled={length < 15}>Continue →</button>
  {showValidationHint && (
    <p className="text-xs text-[var(--muted-foreground)]">
      Tell me a bit more — even one sentence is enough.
    </p>
  )}
  ```
  Show only when user actively tries to submit
  prematurely, not just from typing.

- **Draft state on submit.** Call `useHireDraft().update({
  step: 2, problem: textareaValue })`. The router push
  to `/hire/step/2` happens after the update commits —
  use the hook's promise return if E2-a made it async.

- **No server call yet.** v1 doesn't POST to start a
  hire session. The Genesis-lite call lives in E2-c
  (Listen) which generates the clarifying questions.
  Until then, draft state is purely client-local.

- **Auto-grow textarea.** Use a `ref` + `useEffect` that
  resets `style.height = 'auto'` then sets
  `style.height = scrollHeight + 'px'` on input. ≤ 30
  LOC; don't pull in a library.

- **Accessibility.** Textarea has an explicit
  `<label>` (visually hidden) of `"What do you need
  help with?"`. The headline is `<h1>` so it's the
  page's accessible name; the label is the input's
  programmatic label.

## Hardening briefs implied (Track H)

NOT blocking this PR. Listed for the post-v1 hardening
backlog:

- `mission-enterprise-hire-session-server-start` —
  POST `/api/v1/hire/sessions` creates a server-side
  draft with tenant scope, audit row, quota check.
- `mission-enterprise-hire-input-validation` — server-
  side input shape + content moderation + PII scrub.
- `mission-enterprise-hire-rate-limit` — per-user /
  per-tenant rate limit on hire attempts.
- `mission-enterprise-hire-examples-configurable` —
  operator-configurable example set per workspace.
- `mission-enterprise-hire-input-audit` — audit log
  for every Step 1 input.
