# fix-onboarding-first-hire-name-parser-prefers-named-pattern

**Owner**: claude.
**Authored against**: origin/main `5c4abb23e1f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-first-hire-name-parser-prefers-named-pattern`.
**Source**: e2e-ux-audit (onboarding-hire-an-ai-teammate) findings F2 + F15 — `docs/skycot/ux-audits/2026-05-26-0820/onboarding-hire-an-ai-teammate.md`.

🔴 **P0 — The wizard's own example prompt creates a worker named "A". The investor demo would record a team page where the first hire is literally called "A".**

## Root cause

The gateway helper `name_from_description` at `crates/argo-cli/src/handlers/gateway/routes_onboarding_helpers.rs:232` greedily reads the first capitalised token after whitespace/comma/period. The first-hire page placeholder + hint copy at `web/app/(setup)/onboarding/first-hire/page.tsx:11` literally says:

> "A marketing intern named Maya who drafts blog posts and runs weekly competitive scans."

First capitalised token: **A**. So an operator following the wizard's own example verbatim creates a worker named "A" — visible in the team page header, the audit row, every drill-in.

There is also no UI field to disambiguate. The wizard infers from prose and never shows the user what name it'll pick, so the failure is silent until the team page lands.

Empirical evidence (audit curl):
```
'{"description":"A marketing intern named Maya..."}' → {"name":"A"}
'{"description":"Maya is a marketing intern"}'      → {"name":"Maya"}   (works by accident)
'{"description":"draft marketing posts"}'           → {"name":"Pioneer"} (silent fallback)
```

Two coupled fixes — both ship together because either alone is a half-measure:

1. **Server parser**: prefer `named X` / `called X` / `My X is X` patterns over first-capitalised-token.
2. **Client UI**: add an optional "Their name" field above the textarea with a live preview ("Hire **Maya**" in the submit button), so the operator can see + override the parsed name before clicking.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F2 [P0] — `name_from_description` derives "A" from the wizard's own example prompt
- onboarding-hire-an-ai-teammate F15 [P1] — No "what their name is" field on the first-hire form; no preview of the parsed name

## Fix recipe

**Server-side** (`crates/argo-cli/src/handlers/gateway/routes_onboarding_helpers.rs`):

1. Add a regex-based extractor that tries patterns in order before falling back to first-capitalised-token:
   - `(?i)named ([A-Z][a-z]+)` (capture group 1)
   - `(?i)called ([A-Z][a-z]+)`
   - `(?i)name is ([A-Z][a-z]+)`
   - `(?i)\bI am ([A-Z][a-z]+)` (for first-person descriptions like "I am Maya, a...")
   - First-capitalised-token as last resort
   - "Pioneer" silent fallback only when nothing matches

2. Reject single-letter "names" — if the chosen token is length 1, drop to the next strategy. "A" should never be a valid output.

3. Add unit tests at `crates/argo-cli/src/handlers/gateway/routes_onboarding_helpers.rs` (inline `#[cfg(test)]`):
   - `"A marketing intern named Maya..."` → `"Maya"`
   - `"Bob the SDR"` → `"Bob"`
   - `"draft marketing posts"` → `"Pioneer"` (fallback)
   - `"My new SDR is called Priya"` → `"Priya"`
   - `"A single-word A B C"` → `"B"` (single-letter rejected)

**Client-side** (`web/app/(setup)/onboarding/first-hire/page.tsx`):

1. Add an optional `<input type="text" name="hireName" placeholder="Their name (we'll guess if you skip)" />` above the description textarea.
2. Compute the displayed name: `displayName = hireName.trim() || derivePreview(description)`. `derivePreview` is a client-side mirror of the server parser — same regex order — so the preview matches the server choice.
3. Show the displayed name in the submit button label: `"Hire ${displayName}"` (or "Hire them" if `displayName` is empty).
4. When the form submits, include `name: hireName.trim() || undefined` in the payload — server uses the supplied name verbatim when present, parses from description when absent.
5. Update the server payload type at `web/lib/api/onboarding.ts` (or wherever the first-hire mutation is defined) to accept the optional `name` field.

**Gateway**: extend the `first-hire` request schema to accept an optional `name` field; when present, skip the parser and use it directly (length 1-32 chars, alphabetic + space + hyphen).

## Acceptance criteria

- [ ] `curl -X POST /api/v1/onboarding/first-hire -d '{"description":"A marketing intern named Maya..."}'` returns `{"name":"Maya",...}` (NOT `"A"`).
- [ ] `curl -X POST /api/v1/onboarding/first-hire -d '{"name":"Hana","description":"draft marketing posts"}'` returns `{"name":"Hana",...}` (server respects the supplied name).
- [ ] First-hire page renders a "Their name" input above the textarea.
- [ ] Submit button updates live to show the chosen name as the operator types — "Hire **Hana**".
- [ ] Single-letter token (e.g. "A") never returned as a name by the parser.
- [ ] Server unit tests pass for all 5 cases above.
- [ ] Wizard example placeholder updated to start with the name: "Maya, a marketing intern who drafts blog posts and runs weekly competitive scans." (defense in depth — even if a worker skips the field and the parser regresses, the example still produces "Maya").

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_onboarding_helpers.rs` (name parser + tests)
- `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs` (schema for optional `name` field)
- `web/app/(setup)/onboarding/first-hire/page.tsx` (UI input + preview)
- `web/lib/api/onboarding.ts` (mutation payload type)

## Out of scope

- Duplicate-name detection (separate brief: `fix-onboarding-stat-noun-and-first-hire-name-collision` already covers the "Maya collides with seeded Maya" half)
- Pronoun support on the worker record (separate brief — pair with `fix-worker-activity-empty-state-pronoun-neutral`)

## Effort

~80-150 LOC across gateway + SPA + tests. ~2-3 hours. P0 because the example prompt produces a broken row on first run.
