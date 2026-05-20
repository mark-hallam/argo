# web-a11y-landmark-and-form-labels — landmark-one-main on /compare + real `<label>` on signup/signin

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-a11y-landmark-and-form-labels`.
**Priority**: P2 — wave-2 a11y audit: /compare has no `<main>` landmark; auth forms use `<div className="field-label">` instead of real `<label htmlFor>`.
**Sizing**: ~100-200 LOC (landmark wrap + label refactor + tests).

## Why this brief exists

From the wave-2 a11y audit:

> Landmark / structural failures:
> - `landmark-one-main` on /compare — page has no `<main>` element;
>   screen-reader doesn't know where content starts.
> - `<div className="field-label">` instead of `<label htmlFor>` on
>   signup/signin forms — screen-reader can't associate label with
>   input.

## Goals

1. Wrap /compare page content in a `<main id="main">` (also feeds the skip-link target from the sibling brief).
2. Refactor `<div className="field-label">` → `<label htmlFor={inputId}>` across signup/signin forms.
3. Ensure each input has a matching `id` so `htmlFor` resolves.
4. Visual: zero change — labels look the same but become semantic.

## Steps

1. /compare: wrap the page's content `<div>` with `<main id="main">`.
2. Auth pages: identify the field-label divs + their sibling inputs; refactor to label/input pairs with matching id.
3. Run axe-core to confirm `landmark-one-main` and `label` rules pass.

## may-touch

- `web/app/compare/page.tsx` (wrap with `<main>`)
- `web/app/auth/signup/page.tsx`
- `web/app/auth/signin/page.tsx`
- Optional: `web/components/auth/AuthEditorialShell.tsx` if labels live in the shell

## must-NOT-touch

- The shell's outer structure (this is internal — labels only)
- Other forms (other label issues handled by `web-form-validation-zod-sweep`)

## Acceptance criteria

- [ ] /compare has exactly one `<main>` landmark.
- [ ] All signup/signin form inputs have a corresponding `<label htmlFor>`.
- [ ] axe-core scan reports 0 `landmark-one-main` or `label` violations on the touched pages.
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.
- [ ] Visual diff is null.

## References

- Audit: `docs/skycot/audit/a11y/2026-05-20-wave-2-a11y.md`
- WCAG 2.1 A 1.3.1, 3.3.2
