# Wave-9 Audit #5 — Dark-Mode Per-File Deep Coverage

**Author:** Wave-9 audit agent #5 (read-only).
**Date:** 2026-05-21.
**Scope:** `web/app/**`, `web/components/**`, against the brand-token
contract in `web/app/globals.css`.
**Method:** Static analysis (ripgrep counts + per-file context spot
checks). Server not consulted.
**Predecessor:** `docs/skycot/audit/dark-mode/2026-05-21-wave-6-dark-mode-coverage.md`
(scored adoption 3/10; this audit names the offenders by file).

---

## TL;DR

Wave-6 surfaced the totals (171 hex `#8b5cf6`, 116 `#ec4899`, 122
hardcoded `linear-gradient(`, **0** `var(--brand-gradient)`
references). Wave-9 names the offenders. Pareto rule applies hard:
**the top 10 files contain ~64% of all hex-brand literals**, and
**8 of the top 10 use the same "shadow-token" pattern** (a local
`<style jsx global>` block redeclares `--purple: #8b5cf6;
--pink: #ec4899;` instead of importing `var(--brand-primary)`).
Those 8 files are mechanically refactorable in one sitting; the
remaining offenders need per-callsite review because the literal is
inline JSX (`style={{ color: "#8b5cf6" }}`) or part of a
multi-stop / per-vertical color expression.

The sweep brief
(`web-dark-mode-tenant-token-adoption-sweep`, already in TODO)
should be **split into 2 PRs**, not one — the marketing/auth shells
mechanical rip (PR-A, ~9 files, low review burden) vs the
in-product per-callsite work (PR-B, ~12 files with logic
entanglement). One mega-PR is reviewable in theory but high-blast-
radius across surfaces the operator has design-locked.

---

## 1. Combined offense count — top 25 files

Count = hex `#8b5cf6` / `#ec4899` / `#0891b2` (any case)
   + `rgba(139, 92, 246, …)` / `rgba(236, 72, 153, …)` / `rgba(8, 145, 178, …)`
   + `linear-gradient(` literals.

| Rank | File | Offenses | Lines | Pattern | Mechanical? |
|---|---|---:|---:|---|---|
| 1 | `app/landing/page.tsx` | 36 | 1174 | shadow-token + inline hex | Partial |
| 2 | `app/pricing/page.tsx` | 26 | 810 | shadow-token | YES |
| 3 | `app/for/_components/VerticalLandingShell.tsx` | 25 | 737 | shadow-token + per-vertical color | Partial |
| 4 | `app/docs/page.tsx` | 24 | 725 | shadow-token | YES |
| 5 | `app/customers/page.tsx` | 19 | 488 | shadow-token | YES |
| 6 | `app/changelog/page.tsx` | 19 | 734 | shadow-token | YES |
| 7 | `app/customers/[slug]/page.tsx` | 18 | 270 | shadow-token + inline hex | Partial |
| 8 | `app/(app)/settings/api-keys/styles.tsx` | 17 | — | tagged-template literal | YES |
| 9 | `app/(app)/workflows/page.tsx` | 16 | 602 | shadow-token + multi-stop | Partial |
| 10 | `app/(app)/settings/members/members-styles.ts` | 15 | — | tagged-template literal | YES |
| 11 | `app/(app)/workflows/templates/page.tsx` | 13 | 455 | shadow-token | Partial |
| 12 | `components/skycot/TeammateCard.tsx` | 12 | — | rgba box-shadow + gradient | Partial |
| 13 | `components/skycot/CommandPalette.tsx` | 12 | — | `badgeColor: "#8b5cf6"` data + gradient | NO |
| 14 | `components/auth/AuthEditorialShell.tsx` | 12 | — | shadow-token | YES |
| 15 | `app/(app)/workflows/[verticalId]/[name]/page.tsx` | 12 | 909 | shadow-token + multi-stop | Partial |
| 16 | `components/auth/WizardEditorialShell.tsx` | 11 | — | shadow-token (single-line) | YES |
| 17 | `app/(app)/workflows/templates/[templateId]/page.tsx` | 10 | — | shadow-token | YES |
| 18 | `app/compare/page.tsx` | 9 | — | shadow-token | YES |
| 19 | `app/(app)/skills/page.tsx` | 8 | — | gradient + rgba | Partial |
| 20 | `app/offline/page.tsx` | 7 | — | shadow-token | YES |
| 21 | `app/globals.css` | 6 | — | **canonical definitions** | KEEP |
| 22 | `app/(app)/settings/security/page.tsx` | 6 | — | shadow-token | YES |
| 23 | `components/skycot/BrandLoudHero.tsx` | 4 | — | gradient + rgba | Partial |
| 24 | `app/terms/page.tsx` | 4 | — | shadow-token | YES |
| 25 | `app/security/page.tsx` | 4 | — | shadow-token | YES |

Top-10 subtotal: **213 offenses** of an estimated ~330 total in
`app/` + `components/` (~64% of total surface).

---

## 2. The "shadow-token" anti-pattern

8 of the top-10 files (and 9 total across the audit) contain a
copy-paste block of the form:

```css
:root {
  --purple: #8b5cf6;
  --pink:   #ec4899;
  --coral:  #ff6b6b;
  ...
}
```

inside a `<style jsx global>` tag at the page root. These pages
then reference `var(--purple)` and `var(--pink)` locally — looking
disciplined to a quick reader — but the upstream values are
**hardcoded hex literals duplicated across 9 files** instead of
inheriting from the canonical `globals.css` definitions at
`app/globals.css:117`:

```css
--brand-primary: #8b5cf6;
--brand-pink:    #ec4899;
--brand-gradient: linear-gradient(135deg, #8b5cf6, #ec4899);
```

This is what made wave-6 score adoption 3/10: tokens **exist**, are
**well-named**, are **on the contract**, and are **bypassed in
every marketing surface**. The shadow-token redeclarations also
break dark mode entirely on those pages, because the shadow block
has no `.dark { --purple: ... }` companion — the colors are baked.

Files that re-declare `--purple:` / `--pink:` at the page level:

- `app/landing/page.tsx`
- `app/pricing/page.tsx`
- `app/for/_components/VerticalLandingShell.tsx`
- `app/docs/page.tsx`
- `app/customers/page.tsx`
- `app/customers/[slug]/page.tsx`
- `app/changelog/page.tsx`
- `components/auth/AuthEditorialShell.tsx`
- `components/auth/WizardEditorialShell.tsx` (single-line variant)

**Total `--purple:` redeclarations in the codebase: 9 files.**

---

## 3. Mechanically refactorable vs per-callsite review

### 3.a Mechanical (single sed-able pattern) — PR-A scope

Files where the offense is **only** the shadow-token block + uses
of `var(--purple)` / `var(--pink)`. Refactor = delete the shadow
block, rename `var(--purple)` → `var(--brand-primary)` and
`var(--pink)` → `var(--brand-pink)` (the latter is already defined
on the contract). For the inline `linear-gradient(135deg, #8b5cf6,
#ec4899)` literals, swap to `var(--brand-gradient)`.

Counts of the exact 2-stop brand-gradient string
`linear-gradient(135deg, #8b5cf6, #ec4899)`:
- `app/landing/page.tsx`: 11
- `app/for/_components/VerticalLandingShell.tsx`: 6
- `app/pricing/page.tsx`: 5
- `app/docs/page.tsx`: 5
- `app/(app)/workflows/page.tsx`: 5
- `app/customers/[slug]/page.tsx`: 4
- `app/changelog/page.tsx`: 4
- `app/(app)/workflows/templates/page.tsx`: 4
- All others: 1-3 each
- **Total exact matches: 75** (out of 122 total `linear-gradient(`,
  so ~62% of all gradients are mechanically swappable).

Three-stop matches `linear-gradient(135deg, #8b5cf6, #ec4899,
#ff6b6b)` swap to `var(--brand-gradient-3)`: **13 total**.

**PR-A files (estimated 9 files, ~150 offenses removed):**
1. `app/pricing/page.tsx`
2. `app/docs/page.tsx`
3. `app/customers/page.tsx`
4. `app/changelog/page.tsx`
5. `app/compare/page.tsx`
6. `app/offline/page.tsx`
7. `app/terms/page.tsx`, `app/security/page.tsx`, `app/privacy/page.tsx`
8. `components/auth/AuthEditorialShell.tsx`
9. `components/auth/WizardEditorialShell.tsx`

These are all the design-locked editorial/marketing shells. The
rip is mechanical because every literal lives inside `<style jsx
global>` blocks, never as inline JSX style props.

### 3.b Per-callsite review — PR-B scope

Files where literals leak into the runtime via inline JSX style
props (`style={{ color: "#8b5cf6" }}`), badge data
(`badgeColor: "#8b5cf6"`), or per-vertical color expressions:

- `app/landing/page.tsx` — has 3 inline `<strong style={{ color:
  "#8b5cf6" }}>` for persona name highlights. Refactor needs a
  utility (`<BrandText tone="purple"/>`) because `<strong>` is the
  semantic anchor.
- `components/skycot/CommandPalette.tsx` — `badgeColor:` field in
  the action-type lookup table is consumed by `<Badge>`; needs
  either CSS-var values in the data table or refactor of `<Badge>`
  to accept a token name.
- `app/customers/[slug]/page.tsx` — case-study metric values are
  gradient-text via `-webkit-background-clip: text;` with literal
  gradient string; mechanical swap works but each call also has a
  surrounding "Audiowide" font-family which makes the cluster
  load-bearing.
- `app/for/_components/VerticalLandingShell.tsx` — per-vertical
  accent color is computed from `vertical.id`; cannot be a static
  token, needs a vertical-scoped `--vertical-accent` CSS var.
- `app/(app)/workflows/*` — uses a 3-stop gradient with custom
  intermediate stop colors not on the contract; needs per-callsite
  judgement.
- `components/skycot/TeammateCard.tsx` and
  `components/skycot/BrandLoudHero.tsx` — design-locked in memory
  ("Workers page locked — C2-holographic-showcase"; "Audit page
  locked — B3-enterprise"); any token change needs a visual
  diff against the locked mockup.
- `app/(app)/settings/api-keys/styles.tsx` and
  `members-styles.ts` — tagged-template-literal style modules;
  mechanical-ish, but the styles are scoped to a small surface and
  the operator may prefer a different swap (e.g. accent tokens
  rather than brand tokens, because these are not "loud" surfaces).

**PR-B files (estimated 10-12 files, ~80 offenses):** the workflow
trio + landing inline-JSX + CommandPalette + customer-detail +
TeammateCard + BrandLoudHero + settings styles.

---

## 4. Sizing the sweep brief

The TODO brief `web-dark-mode-tenant-token-adoption-sweep`
currently reads as a single PR. Recommend **split into 2 PRs**:

- **PR-A (mechanical):** 9 files, ~150 literals removed. Estimated
  300-450 lines of diff (deletions outnumber additions). Single
  worker, 1 iteration, no design review needed because the visual
  output is byte-identical when the contract resolves to the same
  hex.
- **PR-B (per-callsite):** 10-12 files, ~80 literals. Estimated
  500-800 lines of diff. Needs design-lock cross-check against the
  6 locked-mockup memories (workers/audit/cohorts/onboarding/
  approvals/auth) before merging. Per-callsite judgement at 4-6
  spots (the vertical-accent expression, the gradient-text Audio-
  wide clusters, the workflow 3-stop intermediate stops, the
  CommandPalette badge-color data).

A single mega-PR is **technically possible** (~1200 LOC, ~21
files, within loc-budget) but risks a one-line typo in PR-B
silently regressing all 6 design-locked pages. PR-A first, deploy
to preview, verify marketing surfaces still look like the locks,
then ship PR-B.

### CI follow-ups

Wave-6 §10.f recommended an ESLint rule banning brand-hex outside
`globals.css`. With PR-A landed, the rule can be added in PR-A
itself (or as a tiny follow-up) because the remaining literals
will all live in PR-B files — which can be added to the allowlist
until PR-B lands. This gives a regression-proof guardrail without
blocking PR-B.

### Definition of done

- 0 occurrences of `--purple:` / `--pink:` page-level
  redeclarations.
- < 20 occurrences of hex `#8b5cf6` / `#ec4899` / `#0891b2` in the
  entire web tree (down from 171 + 116 + ?).
- 0 occurrences of `linear-gradient(135deg, #8b5cf6, #ec4899)` and
  `linear-gradient(135deg, #8b5cf6, #ec4899, #ff6b6b)` outside
  `globals.css`.
- `var(--brand-gradient)` reference count > 50 (currently 0).
- ESLint rule wired and green.

---

## 5. Open questions for the operator

1. **`--brand-pink` vs `--brand-pink`**: contract defines
   `--brand-pink` but no `--brand-purple-shadow-mid` for the
   gradient-text-clipped passages. Confirm the
   `-webkit-background-clip: text` Audiowide metric pattern is
   covered by `var(--brand-gradient)` (it is — already
   `linear-gradient(135deg, #8b5cf6, #ec4899)`).
2. **Per-vertical accents**: the
   `VerticalLandingShell` uses `verticalAccent[vertical.id]` to
   produce per-vertical colors. Confirm whether the vertical-color
   layer is part of the brand-tenant contract or stays as
   per-vertical-pack data.
3. **PR ordering**: should PR-A include the ESLint rule with an
   allowlist for PR-B files, or land the rule in a separate tiny
   PR-C after PR-B?

These questions block authoring the brief, not the audit. Audit
ends here.
