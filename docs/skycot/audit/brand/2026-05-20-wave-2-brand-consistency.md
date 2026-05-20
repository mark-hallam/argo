# SKYCOT Wave-2 Brand Consistency Audit

**Date**: 2026-05-20
**Auditor**: Wave-2 audit agent #5 (read-only)
**Scope**: in-product UI + marketing surfaces — brand recipe fidelity
**Locked brand recipe** (per operator memory + `web/app/globals.css`):

- Display: **Audiowide** (hero + wordmark + some eyebrows)
- Body: **Inter**
- Mono: **JetBrains Mono** (eyebrows + KPI labels)
- Gradient: `linear-gradient(135deg, #8b5cf6, #ec4899)` — purple → pink (NOT cyan)
- Bg: `#fafaff` (warm, with purple + pink radial blobs)
- Ink: `#1e1b4b` (deep indigo)
- Brand-loud register: /landing, /team, /workers/[id], /workflows, /workflows/templates, /cohorts, /skills, /customers, marketing pages
- Brand-restrained register: /audit, /settings, / (approvals), /hire wizard, /onboarding

---

## Overall score

**8 / 10**

Recipe stuck broadly. `web/app/globals.css` declares the canonical `--brand-gradient` purple→pink at the root, and every marketing page + the three pages migrated to `<BrandLoudHero>` are tight. The remaining drift is concentrated in (a) the /onboarding setup wizard shipping with vanilla shadcn theme tokens instead of the locked F1 editorial-center recipe, (b) /workers/[id]/* using a generic shadcn `<WorkerHero>` chrome above the brand-loud `<WorkerPersonalityCard>`, and (c) the locked register for /approvals (Inter-only enterprise-restrained) conflicts with the implemented `<BrandLoudHero>` gradient-banner. Two of these are register mismatches between operator-locked design and shipped implementation; only one is a true colour-drift.

---

## Cyan-gradient drift count

**1** true gradient-rule violation (purple→cyan combo, locked rule is purple→pink):

- `web/app/customers/page.tsx:140` — Harbour Legal customer card uses `linear-gradient(135deg, #0891b2, #8b5cf6)` (cyan → purple). One of 9 distinct per-customer accent gradients on the customers page; the other 8 stay within purple/pink/coral/teal but this one crosses into cyan and reverses the brand direction (purple should always be the leading stop). Recommend `linear-gradient(135deg, #8b5cf6, #14b8a6)` if a non-pink secondary is needed.

**No** true gradient drift elsewhere. The other `#0891b2` mentions are non-gradient uses (category tag fg, role-badge dot, single-character name accent) which are intentional palette diversifiers, not brand-gradient violations.

---

## Pages NOT yet using `<BrandLoudHero>` that should be

The shared component (extracted at `web/components/skycot/BrandLoudHero.tsx`, gradient `135deg #8b5cf6 → #ec4899 → #ff6b6b`, Audiowide title, JetBrains Mono eyebrow, animated shine) is currently consumed by exactly three pages:

```
web/app/(app)/page.tsx              — Approvals
web/app/(app)/team/page.tsx         — Team
web/app/(app)/cohorts/page.tsx      — Cohorts
```

### Brand-loud surfaces NOT yet using BrandLoudHero

1. **`/workers/[workerId]/*` overview tab** — `WorkerLayoutShell` renders the generic `<WorkerHero>` (shadcn theme tokens, no gradient, no Audiowide) above `<WorkerPersonalityCard>`. This is the locked-design "your AI hires are colleagues" surface; the chrome above it is generic.
2. **`/workflows`** — uses the marketplace-hero pattern (Audiowide hero with text-clip gradient + 28px gradient search icon + count pill). This is a deliberate sibling pattern per the operator's Skills design lock (also reused on /skills, /workflows/templates). Decision needed: is this a separate-locked pattern (keep as-is) or should it migrate to BrandLoudHero (consistent banner across brand-loud pages)?
3. **`/skills`** — same marketplace-hero pattern. Same decision.
4. **`/workflows/templates`** — same marketplace-hero pattern. Same decision.

### Register mismatches between lock and implementation

5. **`/` (Approvals)** — operator memory says "Approvals inbox design locked — E-ops-dashboard … **enterprise-restrained register (Inter only)**", but implementation uses `<BrandLoudHero>` (Audiowide + gradient banner). Either the lock memory is stale or the implementation drifted. Worth a 1-line operator confirmation.

---

## Top 5 specific drift fixes (file:line)

| # | File:Line | Issue | Fix |
|---|-----------|-------|-----|
| 1 | `web/app/(setup)/onboarding/layout.tsx:30-50` | The 5-step F1 onboarding wizard ships with vanilla shadcn theme tokens (`bg-[var(--background)]`, `<Sparkles>` icon, no Audiowide). Locked design is "F1 editorial-center across 5 steps … A-style top bar in light theme, centered editorial single column, Inter weight 200 hero typography, the Audiowide wordmark is the only Audiowide on the page". Implementation has zero Audiowide and no purple→pink gradient brand mark. | Adopt `<WizardEditorialShell>` (already exists at `web/components/auth/WizardEditorialShell.tsx`) or import the brand recipe into onboarding/layout.tsx. |
| 2 | `web/app/(app)/workers/[workerId]/WorkerLayoutShell.tsx:77` | Renders `<WorkerHero>` (generic shadcn) above the locked brand-loud `<WorkerPersonalityCard>`. Hero is the first thing the user sees on `/workers/[id]/overview`. | Either bring `<WorkerHero>` into brand-loud register (purple→pink gradient banner, Audiowide name, JetBrains Mono eyebrow) or migrate to a shared `<BrandLoudHero>` variant tuned for per-worker context (eyebrowLabel = role tagline, kpis = today's actions). |
| 3 | `web/app/customers/page.tsx:140` | `linear-gradient(135deg, #0891b2, #8b5cf6)` — cyan → purple, the only true brand-gradient violation in the codebase. Reverses purple-leading rule. | Change to `linear-gradient(135deg, #8b5cf6, #14b8a6)` (purple → teal) for a non-pink Legal accent. |
| 4 | `web/app/(app)/settings/members/page.tsx:179, 428` | Settings is brand-restrained per lock, but the Invite button + a 277/428 CTA use `linear-gradient(135deg, #8b5cf6, #ec4899)`. Single CTA-button accent is borderline; if strict brand-restrained discipline matters, replace with `var(--primary)` solid. | Decision: keep gradient CTA on settings (consistent with global CTAs) OR enforce Inter-only restraint. |
| 5 | `web/app/landing/page.tsx:868` | `<strong style={{ color: "#0891b2" }}>Jamal</strong>` — Jamal's name in landing-hero bubble is colored cyan as a per-worker accent (4 workers, 4 colors). The other 3 are #8b5cf6 / #ec4899 / #14b8a6. Cyan is off-brand. | Swap to `#14b8a6` (teal) or `#ff6b6b` (coral) — staying inside the locked palette. |

---

## Most off-brand surface on the demo path

**`/onboarding/welcome` → `/onboarding/workspace` → `/onboarding/connect-provider` → `/onboarding/pick-vertical` → `/onboarding/first-hire`** (`web/app/(setup)/onboarding/layout.tsx` + the 5 step page files).

This is the *first thing a new operator sees after they sign up*. The locked design is F1 editorial-center with Audiowide wordmark + purple→pink brand mark + warm radial-blob bg + Inter weight 200 hero typography. The shipped implementation:

- No `<link>` to Audiowide Google Font
- No purple→pink gradient anywhere
- No radial-blob bg
- `<Sparkles>` lucide icon (generic) inside a `bg-[var(--primary)]` square instead of the locked SKYCOT brand mark
- `<h1>` uses default tracking-tight semibold instead of weight 200 hero typography

The same 5 steps are also implemented under `web/app/(hire)/hire/step/[step]/page.tsx` (using `<WizardEditorialShell>`, fully brand-correct). So the brand recipe IS available — the `/onboarding` route just never adopted it. This is the single highest-leverage fix: it's the first impression in the signup-to-first-hire flow, it's on the demo path, and the canonical implementation already exists in the same repo.

---

## Per-page summary

| Page | Brand-loud / restrained | Audiowide | Purple→pink gradient | Bg `#fafaff` | Ink `#1e1b4b` | Status |
|------|------------------------|-----------|---------------------|--------------|---------------|--------|
| `/landing` | brand-loud | yes | yes | yes | yes | OK (1 cyan name-accent line 868) |
| `/compare` | brand-loud | yes | yes | yes | yes | OK |
| `/pricing` | brand-loud | yes | yes | yes | yes | OK |
| `/customers` | brand-loud | yes | yes | yes | yes | OK (1 cyan-purple gradient line 140) |
| `/changelog` | brand-loud | yes | yes | yes | yes | OK |
| `/docs` | brand-loud | yes | yes | yes | yes | OK |
| `/auth/signup` (+signin+verify+check-email) | brand-medium | yes (via `<AuthEditorialShell>`) | yes | yes | yes | OK |
| `/` (Approvals) | restrained-per-lock | yes (via `<BrandLoudHero>`) | yes | yes | yes | Register conflict: lock says Inter-only, impl uses brand-loud banner |
| `/team` | brand-loud | yes (via `<BrandLoudHero>`) | yes | yes | yes | OK |
| `/workers/[id]/overview` | brand-loud | no in hero, yes in personality card | partial (only personality card) | yes | yes | DRIFT: WorkerHero is generic shadcn |
| `/workflows` | brand-loud | yes | yes | yes | yes | OK (uses marketplace-hero pattern, not BrandLoudHero) |
| `/workflows/templates` | brand-loud | yes | yes | yes | yes | OK |
| `/workflows/[v]/[n]/build` | brand-loud | yes | yes | yes | yes | OK |
| `/audit` | brand-restrained | none (correct) | none (correct) | yes | yes | OK (correctly restrained) |
| `/skills` | brand-loud | yes | yes | yes | yes | OK |
| `/skills/[skillId]` | brand-loud | yes | yes | yes | yes | OK |
| `/hire/*` | brand-medium | yes (via `<WizardEditorialShell>`) | yes | yes | yes | OK |
| `/onboarding/*` | brand-medium per lock | NO | NO | yes (default) | yes (default) | **MAJOR DRIFT** |
| `/setup` (operator setup wizard) | unspecified | NO | NO | yes (default) | yes (default) | low priority, unspecified |
| `/settings/*` | brand-restrained | none (correct) | gradient on invite-CTA only | yes | yes | OK with minor CTA-gradient leak |

---

## Recommended actions

1. **Operator confirmation needed**: is /approvals brand-loud (current impl) or brand-restrained (lock memory)? The shipped `<BrandLoudHero>` is gorgeous — recommend updating the lock memory to match.
2. **Brief: `mission-brand-onboarding-wizard-editorial`** — apply WizardEditorialShell or its recipe to `web/app/(setup)/onboarding/layout.tsx` + the 5 step pages. Highest-leverage brand fix; first-impression surface.
3. **Brief: `mission-brand-worker-hero-loud`** — upgrade `<WorkerHero>` to brand-loud register (gradient + Audiowide + JetBrains Mono eyebrow) OR fold into a `<BrandLoudHero>` variant.
4. **One-line fix**: change `web/app/customers/page.tsx:140` and `web/app/landing/page.tsx:868` cyan → teal/coral.
5. **Decision**: are /workflows + /skills + /workflows/templates' "marketplace-hero" patterns a locked sibling of `<BrandLoudHero>` (keep) or should everything brand-loud converge on `<BrandLoudHero>` (migrate)? Recommend keep — they're search-led pages where a gradient banner would push the search bar below the fold.

---

End of audit.
