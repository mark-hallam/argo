# rebase-fix-onboarding-first-hire-add-explicit-name-field-honest-parse

Re-implement the original brief
`streams/fix-onboarding-first-hire-add-explicit-name-field-honest-parse.md`
against current `origin/main` HEAD.

## Original PR

PR #2455 (closed for re-author) — `fix(onboarding): add explicit name field
to first-hire wizard`, created 2026-05-27 against a much older `main`.
Conflicted on:

- `web/app/(setup)/onboarding/first-hire/page.tsx` — main has had many
  iterations since: #2477 (persist first-hire prompt across step-back),
  #2493 (soften first-hire copy), #2484 (clickable Help link), #2504
  (wizard polish), #2524 (install-vertical receipt), #2595 (step-pip
  semantics), #2619 (parameterise example prompt by vertical),
  `718fb9f6d` (canonical-roster picker — REVERTED by `58ea1c93f`), and
  the operator-locked MVP-with-Genesis flow.
- `web/lib/onboarding/use-first-hire.ts` + `use-onboarding-draft.ts` —
  see also the operator memory
  `feedback_scene_1_canonical_picker_was_wrong_for_mvp_genesis` —
  per-vertical canonical-picker was reverted; the page is now a Genesis-
  driven first-hire beat. The name-field intent still applies but the
  surrounding shape has changed.
- `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs` — the
  gateway handler has had multiple edits in the same window; the PR's
  optional-name field addition + back-compat fallback to
  `name_from_description` need to land against the current handler
  shape.
- Multiple Playwright specs (`web/tests/e2e/*.spec.ts`) — these had to
  learn the new two-input form contract; rebase should re-derive the
  delta against the post-Genesis-MVP shape.

## Authored against: origin/main `a5df90c41` on 2026-05-29

## Validate before claim: bash scripts/check-brief-freshness.sh rebase-fix-onboarding-first-hire-add-explicit-name-field-honest-parse

## Bug is still load-bearing

Confirmed on `a5df90c41` — the first-hire wizard page still presents a
single textarea for description and relies on a first-token extractor
in the gateway. A buyer typing `She's called Maya and she runs the
blog` still gets a worker named `She's`. The Genesis-MVP shift made
Genesis SSE the primary first-hire flow, but the textarea path is
still the operator-facing wizard surface on `/onboarding/first-hire`.

## Acceptance (same as original brief)

- Dedicated `name` input above the description textarea on
  `/onboarding/first-hire`, with rotating per-vertical placeholder
  ("Maya" / "Marcus" / "Priya" / "Jamal" / "Nori").
- Description copy aligned with the new shape ("Their **role** and
  **what they'll do** — plain English."); the "give them a name" line
  removed.
- `FirstHireRequest` + `FirstHirePayload` carry a `name` field; sent
  in the POST body alongside `description` + `vertical_id`.
- Zustand draft store carries `firstHireName` + `setFirstHireName`,
  persisted via localStorage middleware.
- Gateway `/api/v1/onboarding/first-hire` handler reads optional `name`
  field; older clients omitting `name` fall through to the existing
  `name_from_description` extraction (back-compatible).
- New vitest pin in
  `web/app/(setup)/onboarding/first-hire/__tests__/page.test.tsx`:
  both inputs render, submit disabled until both non-empty, API payload
  carries `{name, description, vertical_id}`, description copy no
  longer claims to accept a name, Zustand draft persists name across
  remounts.
- Updated 5 existing test files (3 Playwright e2e + 2 vitest) to fill
  the new `first-hire-name` input before clicking submit.
- Gateway tests pin (a) explicit `name` honoured verbatim, (b) older
  payloads without `name` extract via legacy heuristic.

## Shape of main on touched files

- `web/app/(setup)/onboarding/first-hire/page.tsx` (current) is the
  post-Genesis-MVP shape with single textarea, vertical-aware
  placeholder, install-vertical receipt rendering, and operator-locked
  copy. The name-input addition goes ABOVE the existing textarea
  block; the description copy edit removes the "name" claim from the
  textarea label.
- `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs`
  currently extracts the worker name via the first-capitalised-token
  heuristic. The rebase adds an optional `name` deserialise + uses it
  verbatim when non-empty.
- The wizard test at `web/__tests__/onboarding/wizard.test.tsx` walks
  the multi-step wizard; the rebase teaches it to fill the new name
  input before submitting from the first-hire step.

## Notes for the worker

- Do NOT re-introduce the canonical-roster picker — `718fb9f6d` was
  reverted by `58ea1c93f` for MVP-with-Genesis. Keep the textarea +
  add a sibling name input.
- The placeholder rotation: per-vertical mapping, default rotation
  cycles through `Maya / Marcus / Priya / Jamal / Nori`. Keep it
  consistent with the post-#2619 example-prompt rotation.
- Back-compat constraint on gateway: older SPA builds still POST
  without `name`. The handler MUST fall through to the legacy
  extractor — don't 400 on missing `name`.
