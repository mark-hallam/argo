# claude-brand-identity-system — Establish SKYCOT brand identity + design tokens (FOUNDATION)

**Owner**: claude. **Depends on**: nothing — FOUNDATION brief, must MERGE within first 2 hours of weekend window.
**Authored against**: origin/main `489e0580` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-brand-identity-system`.
**Source**: §4 of `docs/build-plan/V1_WEEKEND_PLAN.md`.

## Mission alignment

- **Public face of "teammate, not tool" (Claim 1).** Every brand decision is a chance to differentiate SKYCOT from "AI agent #4382."
- **Pre-condition for every UI brief in waves 0–4.** Brand identity is foundational; if it merges late, downstream UI work uses placeholder tokens and we re-do work.

## Scope

P0 FOUNDATION. SKYCOT has a Next.js 15 + Tailwind v4 SPA in `web/` with token-based theming (`web/app/globals.css`) and shadcn-style Radix components in `web/components/ui/`. But:

- No wordmark / logo asset.
- No documented voice & tone guide.
- No locked color palette (only structural tokens like `--background` etc.).
- No locked typography scale (Inter + JetBrains Mono are loaded, but no scale enforced).
- No "Jonny Ive discipline" reference: what's allowed, what's forbidden.

This brief locks all of those, ships them as actual code/tokens, and creates the documentation that future briefs reference.

**Discipline:** materials honesty. Reduction. Care in details. Form follows function — joyfully.

Scope:
- Write `docs/skycot/brand/IDENTITY.md` — the load-bearing voice/tone/identity guide.
- Write `docs/skycot/brand/DESIGN_TOKENS.md` — color/typography/spacing/motion specs.
- Update `web/app/globals.css` to lock brand-accent color (one accent only — operator decision deep-indigo vs warm-coral; default to deep-indigo `oklch(0.65 0.16 256)` if operator hasn't responded by hour 1).
- Update `web/app/globals.css` to lock typography scale: display 72/60/48/36px, heading 30/24/20/18px, body 16/14/12px, mono 14/12px.
- Update `web/app/globals.css` to lock spacing scale (8pt grid).
- Update `web/app/globals.css` motion utilities: `--motion-fast: 150ms`, `--motion-base: 250ms`, `--motion-slow: 400ms`, all `ease-in-out`.
- Ship a wordmark in `web/public/brand/skycot-wordmark.svg` (Inter Display "skycot" in foreground color; favicon variant in 32×32 + 16×16).
- Ship favicon set in `web/public/favicon/` (16x16, 32x32, apple-touch-icon).
- Document forbidden patterns: rainbow gradients, gratuitous shadows, parallax scrolling, attention-stealing entry animations.

LOC: ~400.

## Files you may touch

**Brand documentation:**
- `docs/skycot/brand/IDENTITY.md` (NEW) — voice & tone guide. ≤120 LOC.
- `docs/skycot/brand/DESIGN_TOKENS.md` (NEW) — token specs + design discipline. ≤180 LOC.
- `docs/skycot/brand/README.md` (NEW) — entry point. ≤30 LOC.

**Token implementation:**
- `web/app/globals.css` — extend with brand-accent + typography scale + spacing + motion tokens. ≤80 modified lines.

**Brand assets (SVG only, hand-authored or via simple text-to-SVG):**
- `web/public/brand/skycot-wordmark.svg` (NEW) — Inter Display wordmark. ≤40 LOC.
- `web/public/brand/skycot-wordmark-dark.svg` (NEW) — dark-mode variant. ≤40 LOC.
- `web/public/favicon/favicon.svg` (NEW) — vector favicon. ≤30 LOC.
- `web/public/favicon/icon-192.png` (NEW, generated from SVG) — placeholder; can regenerate later.
- `web/public/apple-touch-icon.png` (NEW, 180×180, generated from SVG).
- `web/app/icon.tsx` (NEW) — Next.js metadata icon route. ≤30 LOC.
- `web/app/apple-icon.tsx` (NEW). ≤30 LOC.

**Storybook (if present) or sample page:**
- `web/app/(app)/brand/page.tsx` (NEW) — internal brand reference page. ≤80 LOC.

## Files you must NOT touch

- `web/components/ui/` existing components — they auto-pick up new tokens. Token-only changes; no component refactor here.
- Backend Rust crates — out of scope.
- Marketing site (`docs/marketing/` or similar) — separate brief.

## Acceptance criteria

- [ ] `docs/skycot/brand/IDENTITY.md` exists with: positioning sentence, voice & tone (5+ examples each direction), forbidden patterns list.
- [ ] `docs/skycot/brand/DESIGN_TOKENS.md` exists with: complete color palette, typography scale, spacing scale, motion tokens, iconography commitments (Lucide-only).
- [ ] `web/app/globals.css` defines `--brand-primary`, `--brand-primary-foreground`, `--font-display`, `--motion-*` tokens.
- [ ] `web/public/brand/skycot-wordmark.svg` exists; renders correctly in light + dark mode.
- [ ] `web/app/(app)/brand/page.tsx` renders a complete brand reference (token swatches, type scale, motion examples, icon grid).
- [ ] `pnpm lint` clean. `pnpm type-check` clean.
- [ ] No regressions in existing pages (`/team`, `/hire`, `/cohorts`).
- [ ] Lighthouse score for `/brand` page ≥ 95.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `claude-brand-identity-system`
- PR title: `brand: lock identity + design tokens + wordmark (claude-brand-identity-system) — FOUNDATION P0`

## Notes for the implementer

- **Jonny Ive discipline:** every element earns its place. If you can remove something and the design still works, remove it.
- **Default to deep indigo** `oklch(0.65 0.16 256)` for brand-primary if operator hasn't responded with override. The operator decision (indigo vs coral) is non-blocking; pick indigo and merge.
- **Wordmark guidance:** Inter Display tracking 0, weight 700 ("Bold"), all lowercase "skycot". No icon glyph; the wordmark IS the logo. Dark-mode variant uses `--brand-primary` foreground.
- **Favicon SVG should be a single brand-primary-colored shape that reads at 16×16.** A diamond, simple geometric mark, or a stylized "s" — pick one and commit.
- **Don't iterate on visual choices in this brief.** Lock + ship. Refinement is a follow-up.
- **Anti-pattern**: do NOT add: gradient backgrounds, drop shadows for decoration, multiple accent colors, photographic hero imagery, stock illustrations. ANYTHING that breaks "Jonny Ive discipline" is out of scope.
- After this brief lands, every downstream brief (signup-flow, onboarding-wizard, marketing-site, etc.) consumes the locked tokens. They MUST NOT introduce new design tokens; they extend the system.
