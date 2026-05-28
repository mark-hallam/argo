# rebase-fix-onboarding-first-hire-name-parser-prefers-named-pattern

Re-implement the original brief
`streams/fix-onboarding-first-hire-name-parser-prefers-named-pattern.md`
against current `origin/main` HEAD.

## Original PR

PR #2342 (closed for re-author) — `fix(onboarding): prefer explicit
first-hire names`, opened 2026-05-25 by codex. Conflicted irretrievably
on four files after several weeks of onboarding-flow churn:

- `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs` — 1
  conflict hunk on the `first_hire_response` body. The current handler
  invokes `name_from_description` + `invoke_genesis(&description, &worker_name)`
  + `voice_preamble_from_description(&worker_name, &description)` (the
  Genesis-driven flow). PR #2342's `validate_supplied_name` insertion
  needs to sit BEFORE the Genesis invocation so the explicit name flows
  through.
- `crates/argo-cli/src/handlers/gateway/routes_onboarding_helpers.rs` —
  1 conflict hunk. Current `name_from_description` is the
  first-capitalised-token heuristic. PR's `regex`-based explicit pattern
  list (`named X` / `called X` / `name is X` / `I am X` / `my <noun> is X`)
  + `first_capitalized_name` fallback. The improvement is still load-
  bearing. Note: the `regex` crate is now allowed in argo-cli, no
  Cargo.toml change needed beyond what main already has.
- `web/app/(setup)/onboarding/first-hire/page.tsx` — 9 conflict markers
  (3 hunks). The file is now `OnboardingShell`-based with heroSegments,
  uses `exampleForVertical(verticalId)` for the placeholder (post-#2619),
  and persists prompt straight to the draft store (post-#2477). PR's
  optional-name-input addition needs to sit ABOVE the existing textarea
  block, not replace the current shape.
- `web/lib/onboarding/use-first-hire.ts` — 6 conflict markers (2 hunks).
  Current `firstHire()` has the demo-mode short-circuit
  (`buildDemoFirstHireResult` mapping vertical → canonical sample
  worker) per `feedback_no_theater_in_demo`, plus a 404 fallback that
  resolves the freshly-hired teammate by name. PR's request shape
  change (add `name` field) needs to thread through both branches.

## Authored against: origin/main `a5df90c41` on 2026-05-29

## Validate before claim: bash scripts/check-brief-freshness.sh rebase-fix-onboarding-first-hire-name-parser-prefers-named-pattern

## Bug is still load-bearing

Confirmed on `a5df90c41`. `crates/argo-cli/src/handlers/gateway/routes_onboarding_helpers.rs:315-335`
still uses the first-capitalised-token heuristic. A demo wizard prompt
that begins with `A marketing intern named Maya ...` (the example copy
from `exampleForVertical("marketing-intern")`) hits the heuristic and
the worker is named `A`. The Genesis SSE path uses the same
`name_from_description` extractor as the input to `invoke_genesis`, so
the bug propagates into the Genesis-driven flow as well.

## Acceptance (same as original brief, adapted to current shape)

- `name_from_description` learns the explicit-pattern list (`named X` /
  `called X` / `name is X` / `I am X` / `my <noun> is X`) BEFORE
  falling through to `first_capitalized_name`. `regex` crate import
  added to `routes_onboarding_helpers.rs` (it is already in the
  workspace; if not, add to argo-cli's Cargo.toml).
- The article `A` (and lowercase `a`) MUST NOT be returned as a name
  — the heuristic filters single-letter tokens and articles.
- Gateway `/api/v1/onboarding/first-hire` accepts an optional `name`
  field. When present + non-empty, `validate_supplied_name` runs:
  - Trim + collapse internal whitespace.
  - Length 1..=32.
  - Chars: alphabetic, space, or hyphen only.
  - Rejection returns a 400 with the field-specific message.
- When `name` is absent or empty-string, fall through to
  `name_from_description(description)` (back-compat with older SPA
  builds that don't send the field).
- Web: optional name input above the description textarea on
  `/onboarding/first-hire`. Stored in the draft store (Zustand persist)
  so step-back doesn't lose it. POST body carries `{ name?, description,
  vertical_id }`.
- The submit-button preview shows the name verbatim when supplied
  (`Hire Maya`) and uses a generic fallback otherwise (`Hire them` —
  current copy).
- Tests:
  - `supplied_name_tests` module in `routes_onboarding.rs` (compact /
    collapse-whitespace / reject-punctuation cases — PR #2342 had
    these).
  - `name_tests` module in `routes_onboarding_helpers.rs` covering
    the 5 explicit-pattern cases + the `A`-rejection regression.
  - Adjacent vitest in `web/app/(setup)/onboarding/first-hire/`
    covering both inputs render + draft persists + payload carries
    `name`.

## Shape of main on touched files

- `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs:246-360`
  — Genesis-driven first-hire handler. Insert the supplied-name
  deserialise + validate block AFTER the `description` and
  `vertical_id` parses, BEFORE the `name_from_description` call so the
  resolved worker name threads through `invoke_genesis` +
  `voice_preamble_from_description`.
- `crates/argo-cli/src/handlers/gateway/routes_onboarding_helpers.rs:312-335`
  — current `name_from_description` returns a `String` (capitalised
  token, falls back to `"Pioneer"`). Refactor: keep `name_from_description`
  signature, internally call `explicit_name_from_description` first,
  then `first_capitalized_name`, then `"Pioneer"`.
- `web/app/(setup)/onboarding/first-hire/page.tsx` — `OnboardingShell`-
  based, uses `useOnboardingDraft` for prompt persistence. Add a
  `firstHireName` field to the draft store and a sibling input. Note
  there is a parallel rebase brief
  `rebase-fix-onboarding-first-hire-add-explicit-name-field-honest-parse`
  that ALSO adds an explicit name input — these two briefs overlap. If
  that one ships first, this rebase can be re-scoped to ONLY the
  backend parser + the `validate_supplied_name` POST acceptance
  (the SPA name input is already there).
- `web/lib/onboarding/use-first-hire.ts` — `FirstHireRequest` +
  `FirstHirePayload` need a `name?` optional field. The demo-mode
  short-circuit (`buildDemoFirstHireResult`) ignores the field (sample
  workers are pre-named); the live path forwards it.

## Notes for the worker

- Check whether `rebase-fix-onboarding-first-hire-add-explicit-name-field-honest-parse`
  has shipped before claiming. If it has, this brief is BACKEND-ONLY
  (parser + validate_supplied_name + back-compat fallback).
- The article-rejection is the load-bearing demo-blocker: every demo
  hire being named `A` is the explicit P0 cited in the original brief's
  UX-AUDIT-T22-20 finding.
- Per `feedback_scene_1_canonical_picker_was_wrong_for_mvp_genesis`,
  do NOT re-introduce a roster-picker. Keep the textarea + add a
  sibling optional name input.
