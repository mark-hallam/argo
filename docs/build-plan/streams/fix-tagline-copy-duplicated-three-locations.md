# fix-tagline-copy-duplicated-three-locations

**Owner**: claude.
**Authored against**: origin/main `80e149df55f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-tagline-copy-duplicated-three-locations`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1028/whole-site-mvp-claims-coverage.md` F22 [P3].

🟡 **P3 — "Hire your first AI teammate" tagline appears in 3 separate places (root layout title, /landing meta, signup metadata). Future tagline iterations need 3-way sync.**

## Root cause

Root document title default, `/landing` title, `/auth/signup` title all carry variations. Three sources of truth.

## Findings this brief closes

- whole-site-mvp-claims-coverage F22 [P3] — tagline copy duplicated

## Fix recipe

1. **Extract** to `web/lib/marketing-copy.ts`:
   ```ts
   export const SKYCOT_TAGLINE = "Hire your first AI teammate";
   export const SKYCOT_NAME = "SKYCOT";
   ```
2. **Replace literal occurrences** in:
   - `web/app/layout.tsx` (root)
   - `web/app/landing/page.tsx` (or marketing layout)
   - `web/app/auth/signup/page.tsx` (metadata)
3. **Lint rule** (optional) flags duplicate tagline strings.
4. **Web test** asserts the imported constant matches expected.

## Acceptance criteria

- [ ] `web/lib/marketing-copy.ts` exports the tagline constant.
- [ ] All 3 sites reference the constant.
- [ ] `grep '"Hire your first AI teammate"' web/` returns only the constant definition.
- [ ] Web test covers the constant.

## May-touch

- `web/lib/marketing-copy.ts` (NEW)
- `web/app/layout.tsx`
- `web/app/landing/page.tsx` (or marketing layout)
- `web/app/auth/signup/page.tsx`
- `web/tests/marketing-copy-canon.test.ts` (NEW)

## Out of scope

- Tagline rewording (operator-decision)
- Other duplicated marketing strings (full sweep deferred)

## Effort

~25-40 LOC + 1 test. ~30min.
