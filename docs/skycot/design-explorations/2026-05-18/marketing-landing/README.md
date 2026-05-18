# Marketing landing — design exploration (2026-05-18)

Optometrist method: wildly different mockups, operator picks which feels right, then narrow down.

## Round 1 — five wildly varied design languages (extremes)

| File | Design language |
|---|---|
| [option-A-editorial.html](./option-A-editorial.html) | Editorial / Magazine cover |
| [option-B-industrial.html](./option-B-industrial.html) | Industrial / Dev-tool / Terminal |
| [option-C-apple-minimal.html](./option-C-apple-minimal.html) | Apple-minimal / Silent confidence |
| [option-D-friendly-illustrated.html](./option-D-friendly-illustrated.html) | Hand-drawn / Friendly / Illustrated |
| [option-E-brutalist.html](./option-E-brutalist.html) | Brutalist / Statement piece |

**Operator picked: D (friendly/illustrated).** Refinements requested: Canva-inspired colors, enterprise appeal, Melbourne (not Sydney) origin.

## Round 2 — three palette variants on the D direction

| File | Palette |
|---|---|
| [option-D1-canva-coral-enterprise.html](./option-D1-canva-coral-enterprise.html) | Coral / peach / yellow |
| [option-D2-canva-teal-enterprise.html](./option-D2-canva-teal-enterprise.html) | Teal / mint / navy |
| [option-D3-canva-gradient-vibrant.html](./option-D3-canva-gradient-vibrant.html) | Purple → pink → coral gradient |

**Operator response: use the official SKYCOT brand colors instead** (Electric Violet `#8B5CF6` → Deep Cyan `#0891B2` logo gradient), and prefer light backgrounds. Brand guide loaded — full palette + Audiowide typography + usage rules now applied.

## Round 3 — three brand-correct variants (this round)

All three follow the official SKYCOT Brand Style Guide (April 2026):

- **Light page** as base (operator preference)
- **Logo plate** in nav: gradient text on dark bar (brand-correct, matches PDF page 2)
- **Audiowide** for headings and display text
- **Inter** for body and UI
- **Brand gradient** `linear-gradient(135deg, #8B5CF6, #0891B2)` appears on:
  - The logo plate
  - Primary CTA fills (white text on gradient — meets WCAG AA)
  - Dedicated dark "feature moments" within each variant
- **Solid Electric Violet + Deep Cyan** as accent colors on light text (since "gradient text on light backgrounds" is a brand "don't")
- **Extended palette** (`#D946EF` Fuchsia, `#3B82F6` Ocean) used in illustrations
- **Tagline** "AI-Enabled Robotics" (from brand guide)
- **Footer**: "Built in Melbourne, Australia"
- **Enterprise scaffolding**: customer logo row, 4 compliance badges (SOC 2, GDPR, SSO, SQLCipher), VP testimonial

### What's different — three approaches to the "how much gradient" dial

| File | Lean | Dark/Gradient surface area | Energy |
|---|---|---|---|
| [option-D4-brand-premium.html](./option-D4-brand-premium.html) | **Premium / Restrained** | Minimal — only nav logo plate + hero art card + close-CTA section | Quiet, confident, Linear/Notion-feel |
| [option-D5-brand-editorial-bands.html](./option-D5-brand-editorial-bands.html) | **Editorial / Balanced** | Medium — hero is split light+dark, plus a dedicated dark "how it works" band | Balanced, magazine-style, most varied |
| [option-D6-brand-gradient-art.html](./option-D6-brand-gradient-art.html) | **Gradient-as-Art / Boldest** | High — big dark hero canvas + dark cards-with-art tops + dark testimonial section + dark close | Vibrant, "Stripe-feel", most likely to go viral |

### Layout signatures per variant

- **D4** — Hero illustration sits inside a single dark rounded panel (right side); rest of page is white with subtle violet/cyan accents. The most enterprise-tasteful.
- **D5** — Hero is half-light / half-dark side-by-side; a "How it works" dark band in the middle of the page gives the gradient its full proper treatment.
- **D6** — Hero has a large dark "canvas" with abstract gradient blobs as art; each feature card has a mini gradient-art header; testimonial is a dark gradient-art container with KPIs.

## How to pick — round 3

Open all three. Five-second gut. Tell me one of:

1. **"D4 — restrained"** → narrow with 2-3 minor variations (typography weight, hero illustration angle, CTA shape)
2. **"D5 — editorial bands"** → narrow on the band approach
3. **"D6 — gradient art"** → narrow on the gradient-art density
4. **"Hybrid — D5's hero structure with D6's testimonial section"** → I'll merge
5. **"None — go further toward [direction]"** → another round

Once a winner emerges:

- I author the implementation brief for the swarm to rebuild `marketing/` to match
- I produce the brand-consistent design pass on the **audit page** (the in-product showpiece — the screenshot people will share)
- Then onboarding, workers, cohorts, auth pages, settings, marketplace
