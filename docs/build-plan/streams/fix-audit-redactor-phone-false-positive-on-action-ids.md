# fix-audit-redactor-phone-false-positive-on-action-ids

**Owner**: claude.
**Authored against**: origin/main `b77864a000a9738c80f235fc2605d192197cd5c7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-audit-redactor-phone-false-positive-on-action-ids`.
**Source**: e2e-ux-audit T0819 — `docs/skycot/ux-audits/2026-05-27-0819/scene-1-and-2-claim-1-and-2.md` F4 [P0].

🔴 **P0 — PII (Personally Identifiable Information) redactor mangles the action_id itself. On `/audit/action/act_018f67d0-0000-7000-8000-000000000001_000` the Output panel renders: "Detail for action act_018f67d0-0000-7000-8000-000\[PHONE\]_000 is not currently available." The 12-digit zero suffix inside the UUID trips the phone-number redactor. Camera reads `[PHONE]` inside an action id — customer-visible bug that breaks the "you can copy this id and find it again" trust promise.**

## Root cause

The SPA-side PII redactor scans free-text fields for digit runs matching a phone-number-shaped regex (likely `\d{10,12}` or similar). UUIDs (Universally Unique Identifiers — 128-bit canonical hex strings) contain a 12-character segment after the third hyphen which is all-digits when the original UUID had low entropy in that band (e.g. `018f67d0-0000-7000-8000-000000000001`). The redactor matches the 12-digit run and replaces it with `[PHONE]`.

Compounding inconsistency: the SAME action id renders unredacted in the Hero block of the same page (`act_018f67d0-0000-7000-8000-000000000001_000`). So the camera sees the same id rendered two different ways on one screen. This means redaction is applied to free-text content (e.g. `output.text`, `inputs.message`) but not to copy-id reference strings (Hero) — the redactor is correctly scoped at the Hero block but is being applied to the Output block's empty-state copy where it shouldn't.

Action ids are NOT PII — they are opaque tenant-scoped identifiers. The redactor library lives at `web/lib/redaction/` (or similar — grep for `[PHONE]` to locate). Likely a single function that's being called on every text field including the empty-state copy in the Output panel.

Evidence:
- `/audit/action/act_018f67d0-0000-7000-8000-000000000001_000` body contains `[PHONE]` inside the Output section
- Hero block of the same page shows the id unredacted
- Activity-row drill-in for any sample worker produces this pattern

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F4 [P0] — PII redactor mangles the action id itself: `act_018f67d0-0000-7000-8000-000[PHONE]_000`

## Fix recipe

1. **Locate the redactor.** `rg '\\[PHONE\\]' web/lib/` and `rg 'PHONE' web/lib/redaction/` to find the replacement constant and the regex.
2. **Exclude UUID-shaped tokens from the phone-redactor regex.** Two options:
   - **(a) preferred** — add a UUID guard: skip redaction if the digit run is preceded or followed by a hyphen-separated hex block (regex: `(?<!-[0-9a-f]{4}-)\d{10,12}(?!-[0-9a-f]{4}-)`).
   - **(b) safer** — only apply the phone-redactor to fields explicitly tagged as free-text. Add a `redactFreeText(s: string)` helper used only in `output.text`, `inputs.message`, `notes`, etc., and a `redactIdentifier(s: string)` that just passes through. Update audit-detail rendering to pick the right helper per field.
3. **Audit-detail rendering hygiene** at `web/app/(app)/audit/action/[actionId]/page.tsx:138-156`: the empty-state copy ("Detail for action {id} is not currently available") should use `redactIdentifier` (no-op), not the free-text redactor.
4. **Vitest pin** at `web/lib/redaction/redactor.test.ts`: assert `redactFreeText("Call me at 5551234567 now")` → contains `[PHONE]`; `redactIdentifier("act_018f67d0-0000-7000-8000-000000000001_000")` → unchanged.
5. **Optional polish**: rename the redactor's replacement token to `[redacted-phone]` for accessibility/screen-reader clarity. Out of scope for this brief.

## Acceptance criteria

- [ ] `/audit/action/act_018f67d0-0000-7000-8000-000000000001_000` empty-state copy renders the full action id with no `[PHONE]` substitution.
- [ ] Hero block and Output block render the SAME action id consistently.
- [ ] Free-text phone redaction still works for genuine phone strings in `output.text` etc. (verified by vitest).
- [ ] No regression on the existing `fix-spa-error-pages-and-audit-redactor` guards.

## May-touch

- `web/lib/redaction/` (the redactor regex or split into two helpers)
- `web/app/(app)/audit/action/[actionId]/page.tsx:138-156` (call the right helper for empty-state copy)
- `web/lib/redaction/redactor.test.ts` (new vitest fixture)

## Out of scope

- DecisionChain + Provenance empty-state rendering (covered by `fix-audit-detail-render-empty-decision-chain-and-provenance-fallback`)
- The fabricated 8-step generic chain alongside the empty-state (covered by `fix-activity-row-audit-detail-synthesized-fixture-theatre`)
- Server-side redaction in gateway (separate brief if needed)

## Effort

S — ~30-60 LOC. ~1 hour. P0 because the camera reads `[PHONE]` inside an audit id on the v1.0 demo MP4 drill-in path.
