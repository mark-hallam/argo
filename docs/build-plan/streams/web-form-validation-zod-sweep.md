# web-form-validation-zod-sweep — apply zod to the 15 forms missing validation

**Owner**: claude.
**Authored against**: origin/main `546e3072` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-form-validation-zod-sweep`.
**Priority**: P1 — wave-4 forms audit found 15 of 29 forms with no validation schema (zod and react-hook-form installed but only 1 callsite uses zod). Highest-impact fix: the Worker Settings 5 sections (most-likely-clicked on camera, currently silent-broken with double-submit possible).
**Sizing**: ~500-800 LOC (~20-30 LOC per form × 15 forms + shared zod-resolver wiring + tests).

## Why this brief exists

From the wave-4 forms audit (`docs/skycot/audit/forms/2026-05-20-wave-4-forms.md`):

> **Headline finding**: zod and react-hook-form are installed but
> only ONE file uses zod (`web/lib/auth/signup-schema.ts`). Every
> other form rolls its own trim/regex/min-char checks. No
> centralized validation strategy.
>
> **Highest-impact fix before tomorrow's demo: the Worker Settings
> 5 sections** (one shared mutation-state refactor + a single
> "Saved" toast fixes all 5; it's the most-likely-clicked surface
> on camera; and the current behavior is silent-broken — the worst
> kind for video).

## Goals

1. Centralize form validation on zod + react-hook-form (both already in package.json).
2. Apply to the top 5 demo-risk forms first: Worker Settings 5 sections (Channels / Quiet hours / Budget / Autonomy / Approval-policy).
3. Then the next 10: Settings/Members invite modal, Worker AddFact, Hire AdjustDrawer, Worker edit responsibilities, support ticket, etc.
4. Every form gets:
   - zod schema for validation
   - Inline error display next to the offending field (not just toast)
   - disable-while-submitting (no double-submit)
   - "Saved" success toast OR inline confirmation
   - Esc closes modals; Browser-back preserves draft where it makes sense

## Steps

1. Add a shared `web/lib/forms/use-zod-form.ts` (thin wrapper around react-hook-form + zodResolver).
2. Migrate Worker Settings 5 sections first (highest demo risk).
3. Migrate the other 10 forms.
4. Add tests for each schema (Zod parse on valid + invalid inputs).
5. Verify behaviour: submit invalid → inline error shows; submit valid → submit button disables → toast confirms.

## may-touch (top-5 priority)

- `web/app/(app)/workers/[workerId]/settings/page.tsx` + sub-section components (Channels / Quiet hours / Budget / Autonomy / Approval-policy)
- `web/app/(app)/settings/members/page.tsx` (invite modal)
- `web/components/AddFactDialog.tsx`
- `web/app/(hire)/hire/_components/AdjustDrawer.tsx`
- Shared: `web/lib/forms/use-zod-form.ts` (new)

## may-touch (secondary, next 10)

- Per the wave-4 forms audit report's full inventory.

## must-NOT-touch

- `web/lib/auth/signup-schema.ts` (already uses zod — reference pattern, don't break)
- Public marketing pages (no forms there worth migrating)

## Acceptance criteria

- [ ] Worker Settings 5 sections all use zod + react-hook-form via the shared `use-zod-form` hook.
- [ ] Each migrated form has: schema + inline error display + disable-while-submitting + success toast/confirmation.
- [ ] Esc closes modals; double-submit no longer possible (button disables on in-flight).
- [ ] tsc clean, lint clean, every new schema has at least 2 tests (valid + invalid).
- [ ] Wave-4 forms-audit re-run shows: migrated forms move from "trim-only" to "schema-validated"; the "double-submit possible" count drops by ≥5.

## References

- `docs/skycot/audit/forms/2026-05-20-wave-4-forms.md` (the motivating audit + full 15-form inventory)
- `web/lib/auth/signup-schema.ts` (the one existing zod callsite — reference pattern)
