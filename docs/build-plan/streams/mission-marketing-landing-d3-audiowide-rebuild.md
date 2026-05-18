# mission-marketing-landing-d3-audiowide-rebuild — rebuild marketing/ landing page to match the chosen D3+Audiowide design

**Owner**: claude. **Sizing hint**: large UI work, multiple components, single page.
**Depends on**: `mission-marketing-mdx-turbopack-fix` MERGED (so `pnpm dev` works cleanly in `marketing/`).
**Authored against**: origin/main on 2026-05-18.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-marketing-landing-d3-audiowide-rebuild`.
**Source**: 2026-05-18 design exploration. Operator picked `docs/skycot/design-explorations/2026-05-18/marketing-landing/option-D3-with-audiowide.html` after a 4-round optometrist-method design exploration.

## Mission alignment

- **Commercial usability** (CLAUDE.md priority #5) — the marketing landing is the first impression a prospective customer gets. Must look every bit as polished as the chosen mockup.
- **🎯 Operator brand promise (2026-05-18)**: "the UX and the visible UI must be epic, this must go viral."

## Scope

P1 — substantial UI rebuild. Replace the existing `marketing/app/page.tsx` and supporting components to render the design at the reference file pixel-for-pixel (allowing for minor responsive adjustments). Keep marketing routing (`/features`, `/pricing`, `/customers`, `/about`, `/blog`, `/changelog`) intact — only the homepage `app/page.tsx` and the components it uses change.

## Reference

**Authoritative source for visual + copy**: `docs/skycot/design-explorations/2026-05-18/marketing-landing/option-D3-with-audiowide.html`

This is a self-contained HTML file with inline CSS and SVG. The worker should open it in a browser (`open <path>`), read the source, and rebuild as Next.js / Tailwind components matching the rendered output.

## Detailed spec (what to build)

### 0. Typography setup

- **Audiowide** loaded via Next.js Google Fonts integration (`import { Audiowide } from "next/font/google"`)
- **Inter** loaded the same way (`import { Inter } from "next/font/google"`)
- Both wired into the root `marketing/app/layout.tsx` so they're available app-wide
- CSS variables `--font-audiowide` and `--font-inter` set on `<html>` element
- Tailwind `tailwind.config.ts` extended with `fontFamily.audiowide: ['var(--font-audiowide)']` and `fontFamily.inter: ['var(--font-inter)']`

### 1. Color tokens (Tailwind theme extension)

Add to `tailwind.config.ts` (or equivalent in marketing's Tailwind v4 setup):

```ts
colors: {
  ink: '#1E1B4B',
  muted: '#6B7280',
  line: '#ECE9F5',
  // Brand palette
  'brand-purple': '#8B5CF6',
  'brand-purple-deep': '#6D28D9',
  'brand-pink': '#EC4899',
  'brand-coral': '#FF6B6B',
  'brand-yellow': '#FCD34D',
  'brand-teal': '#14B8A6',
  // Soft tints
  'soft-purple': '#F3E8FF',
  'soft-pink': '#FCE7F3',
  'soft-yellow': '#FFF0D6',
  'soft-teal': '#D2F5ED',
},
```

Background of page is `#FAFAFF` (set as body bg in `app/layout.tsx`).

### 2. Components to build (one file each)

In `marketing/components/landing/`:

- **`SiteNav.tsx`** — sticky nav with backdrop blur. Gradient brand-mark square (32×32, rounded-10) + "SKYCOT" wordmark in Audiowide (font-size 20px, letter-spacing 0.12em). 5 nav links. "Sign in" ghost + "Hire your first worker" gradient pill.
- **`Hero.tsx`** — two-column layout. Left: eyebrow pill with pulse dot animation, h1 (Audiowide, 72px, ALL CAPS, three lines, middle line uses purple→pink→coral gradient text via background-clip, last line "(SERIOUSLY.)" has a yellow underline highlight), sub-copy (Inter 21px), CTA row (gradient primary + white-with-border ghost), reassurance row (4 checkmarks). Right: SVG illustration (see below).
- **`HeroIllustration.tsx`** — inline SVG with the MAYA worker character + 4 floating action chips + 3 dashed connector paths + 3 sparkle dots + 3 background gradient blobs. Copy verbatim from the reference HTML.
- **`LogosRow.tsx`** — "Trusted by 100+ ops, HR & revenue teams" label with `100+` in gradient text + 6 placeholder customer logos in mixed typography (regular, italic Georgia, monospace, allcaps).
- **`Features.tsx`** — kicker pill ("What you get" in Audiowide), section h2 (Audiowide, ALL CAPS, "THREE THINGS SOFTWARE HAS NEVER DONE BEFORE."), sub-copy, then 3 feature cards in a grid. Each card has a 4px top stripe gradient (c1 purple→pink, c2 coral→yellow, c3 teal→purple), 64×64 soft-tint icon container with inline SVG icon, h3 (Inter 26px 800), description (Inter 16px muted), stat-tag pill.
- **`ComplianceStrip.tsx`** — white rounded card with 1.5-fr headline column + 4 badge columns. Headline uses Audiowide ALL CAPS ("BUILT ENTERPRISE-READY, NOT BOLTED ON.") with gradient on "ENTERPRISE-READY". Each badge: 44×44 rounded square with 2-color gradient icon (SOC2 / GDPR / SSO / AES256) + name + detail line.
- **`Testimonial.tsx`** — dark gradient card (ink-deep → purple-deep at 135deg). Left: "From the field" label in Audiowide pink-tinted, blockquote in Audiowide mixed-case 26px, "who" row with avatar + name + title. Right: 4-cell KPI metrics grid (white-on-dark cards), each KPI number in Audiowide gradient text (53% / $8.2k / 4.2x / 100%) with label below.
- **`CloseCta.tsx`** — large gradient banner (purple→pink→coral). H2 Audiowide ALL CAPS "HIRE YOUR FIRST TEAMMATE TODAY." + sub-copy + 2 CTAs (white primary "Hire your first worker →" + transparent secondary "Talk to our team"). Yellow + teal decorative circles in corners (low opacity, overflow hidden).
- **`SiteFooter.tsx`** — copyright row + "Built in Melbourne, Australia" with gradient heart icon.

### 3. Page composition

`marketing/app/page.tsx` becomes a thin shell that renders the above components in order:

```tsx
<SiteNav />
<Hero />
<LogosRow />
<Features />
<ComplianceStrip />
<Testimonial />
<CloseCta />
<SiteFooter />
```

Plus the viewport background blobs (fixed-position, low-opacity radial gradients) in `app/layout.tsx` or a `BackgroundBlobs.tsx` component.

### 4. Responsive

The reference design is desktop-first (1280px max-width). Add responsive variants:

- **`<1024px` (tablet)**: hero becomes single column (illustration moves below text), section h2 sizes scale down (72→56 hero, 48→36 section, 60→44 close), nav links collapse to a hamburger (use a simple disclosure + Tailwind Headless UI or similar), compliance strip stacks badges 2×2
- **`<768px` (mobile)**: all single-column, hero h1 down to 40px, feature cards stack vertically, KPI metrics stack 2-up then 1-up at 414px, footer stacks
- **`<414px` (small mobile)**: minimum legible sizes; ensure no horizontal scroll

Test at: 1920, 1440, 1280, 1024, 768, 414, 375 px.

### 5. Animations

- Pulse dot on hero eyebrow: CSS keyframe, 2s ease-in-out infinite, scales box-shadow from 0 0 0 4px → 0 0 0 8px
- CTA buttons: 0.15s transition on hover, translateY(-2px) on primary, translateY(-1px) on small primary
- Feature cards: 0.2s transition, translateY(-6px) on hover with stronger box-shadow
- Honor `prefers-reduced-motion: reduce` — pause animations + remove hover transforms

### 6. Accessibility

- All gradient text must have visible focus states (the underlying solid color should be discoverable via keyboard nav)
- All CTAs are real `<a>` or `<button>` elements with proper roles + aria-labels
- Hero illustration is decorative — `<svg role="img" aria-hidden="true">` (or move it inside a `<picture>` with empty alt)
- Lighthouse a11y score must be ≥95

### 7. Performance

- Audiowide is one weight (regular 400). Load with `display: 'swap'`.
- Inter: only load the weights we use (400/500/600/700/800/900).
- All SVGs inline (already in reference).
- No external image assets needed for v1 — placeholder customer logos are text.
- Bundle delta target: ≤30KB gzipped beyond what the current marketing/ page has.

## Files you may touch

- `marketing/app/page.tsx` — shell. ≤30 modified LOC.
- `marketing/app/layout.tsx` — font loading + body bg. ≤20 modified LOC.
- `marketing/components/landing/SiteNav.tsx` (NEW). ≤80 LOC.
- `marketing/components/landing/Hero.tsx` (NEW). ≤120 LOC.
- `marketing/components/landing/HeroIllustration.tsx` (NEW). ≤200 LOC (SVG-heavy).
- `marketing/components/landing/LogosRow.tsx` (NEW). ≤60 LOC.
- `marketing/components/landing/Features.tsx` (NEW). ≤180 LOC.
- `marketing/components/landing/ComplianceStrip.tsx` (NEW). ≤120 LOC.
- `marketing/components/landing/Testimonial.tsx` (NEW). ≤140 LOC.
- `marketing/components/landing/CloseCta.tsx` (NEW). ≤80 LOC.
- `marketing/components/landing/SiteFooter.tsx` (NEW). ≤40 LOC.
- `marketing/components/landing/BackgroundBlobs.tsx` (NEW). ≤30 LOC.
- `marketing/app/globals.css` — minor additions (keyframes for pulse). ≤30 modified LOC.
- `marketing/tailwind.config.ts` — extend theme with colors + fonts. ≤40 modified LOC.
- `marketing/package.json` — already includes `next/font` (Next.js 15.5+ has it built-in). Verify; add if missing.

## Files you must NOT touch

- The marketing site's OTHER routes (`/features`, `/pricing`, `/customers`, `/about`, `/blog`, `/changelog`) — out of scope for this brief. Their layouts may still use the OLD landing components; that's fine, they'll be migrated in a follow-up.
- The `web/` SPA — different app, different design system.
- The design exploration HTML files in `docs/skycot/design-explorations/` — these are read-only reference.
- Brand identity files in `~/.claude/CLAUDE.md` — global preferences, not a brief edit target.

## Acceptance criteria

- [ ] `cd marketing && pnpm dev` boots cleanly and the homepage renders at `http://localhost:3001/`.
- [ ] Visually matches the reference HTML pixel-for-pixel at 1920×1080 (allowing ≤5px tolerance).
- [ ] Audiowide loads via Google Fonts and renders correctly (verify no font-flash; check `display: 'swap'` is honored).
- [ ] At 768px viewport, hero is single column, all text legible, no horizontal scroll.
- [ ] At 414px viewport, all sections render usably.
- [ ] Lighthouse a11y ≥95 on `/` page.
- [ ] Bundle delta ≤30KB gzipped over current main marketing build.
- [ ] `pnpm typecheck && pnpm lint && pnpm test && pnpm build` all clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-marketing-landing-d3-audiowide-rebuild`
- Title: `feat(marketing): rebuild landing to D3+Audiowide design (mission-marketing-landing-d3-audiowide-rebuild)`
- Worker verification: pnpm checks + manual viewport walkthrough at 5+ widths + Lighthouse run.

## Hidden constraints

- **The reference HTML is authoritative.** If the worker disagrees with a design choice (e.g. "ALL CAPS feels aggressive on mobile") — flag it in the PR body as a boundary deviation, don't silently change. The operator picked this exact mockup; deviations need explicit sign-off.
- **Audiowide only has one weight.** Don't try `font-weight: 700` on Audiowide text — it has no effect. Use the font size + letter-spacing to create hierarchy.
- **The page must still look excellent without the Audiowide font** if it fails to load — Inter as fallback should be acceptable. `font-display: swap` handles this.
- **The customer logos are placeholders** ("Nimbus.io" / "Hartwell & Co." etc.). Keep as text placeholders for now; real logos drop in via a follow-up brief once we have signed customer references.
- **The hero illustration's MAYA character is the brand mascot for the landing.** Future pages (audit, workers, etc.) will introduce other worker characters following the same visual language (rounded-square head + antenna + gradient name tag).
- **Closes the operator's chosen marketing landing direction after a 4-round optometrist-method design exploration** (round 1: 5 wild design languages; round 2: 3 Canva palettes on D; round 3: 3 brand-correct variants; round 4: 3 D3-narrowing variants; final pick: option-D3-with-audiowide.html).
