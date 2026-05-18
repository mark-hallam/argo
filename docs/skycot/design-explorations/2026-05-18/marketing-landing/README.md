# Marketing landing — design exploration (2026-05-18)

Optometrist method: wildly different mockups, operator picks which feels right, then narrow down.

## Round 1 — five wildly varied design languages (extremes)

| File | Design language | Voice | Trade-off |
|---|---|---|---|
| [option-A-editorial.html](./option-A-editorial.html) | Editorial / Magazine cover | Literary, confident, sparse | Slower to read |
| [option-B-industrial.html](./option-B-industrial.html) | Industrial / Dev-tool / Terminal | Technical, honest, no fluff | Cold to non-technical buyers |
| [option-C-apple-minimal.html](./option-C-apple-minimal.html) | Apple-minimal / Silent confidence | One huge sentence per scroll | Demands a polished product |
| [option-D-friendly-illustrated.html](./option-D-friendly-illustrated.html) | Hand-drawn / Friendly / Illustrated | Like a teammate over coffee | Can feel "small" for enterprise |
| [option-E-brutalist.html](./option-E-brutalist.html) | Brutalist / Statement piece | Loud, asymmetric, confident | Hostile to risk-averse buyers |

**Operator picked: D (friendly/illustrated).** Refinements requested:
- Canva-inspired color palette
- Enterprise appeal — keep warmth but add trust signals
- Melbourne, Australia origin (not Sydney — original mockups had this wrong)

---

## Round 2 — three variants in the D direction (this round)

Each one carries the same enterprise scaffolding (customer logo row, compliance badges, VP-level testimonial, "Built in Melbourne, Australia") but with a different Canva color palette.

| File | Color palette | Lean | Best for |
|---|---|---|---|
| [option-D1-canva-coral-enterprise.html](./option-D1-canva-coral-enterprise.html) | **Coral / peach / yellow** (warmest) | Most approachable | SMB-to-mid-market buyers who want warmth + reassurance |
| [option-D2-canva-teal-enterprise.html](./option-D2-canva-teal-enterprise.html) | **Teal / mint / navy** (balanced) | Cool but friendly | Mid-to-large enterprise buyers evaluating against Notion/Linear |
| [option-D3-canva-gradient-vibrant.html](./option-D3-canva-gradient-vibrant.html) | **Purple → pink → coral gradient** (boldest) | Most viral / creative | Standing out + going viral; the "Canva" feel most directly |

### What's the same across all 3

- Refined SVG illustrations (geometric central worker + floating action chips)
- Hero with eyebrow pill + grad headline + sub + dual CTAs + reassurance row
- "Trusted by ops, HR and revenue teams at" logo row (6 placeholder companies)
- 3 feature cards with stat tags
- 4 compliance badges (SOC 2 Type II, GDPR, SSO + SAML, SQLCipher AES-256)
- VP-level testimonial ("Riley Kovacs · VP Customer Success · Nimbus.io")
- Big CTA close section
- Footer reads "Built in Melbourne, Australia"

### What's different between them

| Element | D1 (Coral) | D2 (Teal) | D3 (Gradient) |
|---|---|---|---|
| Primary CTA | Rounded pill, coral | Rounded square, teal | Pill, purple→pink gradient |
| Hero headline | "Hire a teammate. Not another tool." | "The first AI teammate you can actually audit." | "Don't buy another tool. Hire a teammate. (seriously.)" |
| Card hover | Lifts up | Lifts + teal border glow | Lifts + colored top stripe |
| Testimonial section | Navy card with compliance grid | Centered quote with avatar | Gradient card with 4 KPI metrics |
| Background | Plain warm cream | Plain off-white | Soft purple/pink blob gradients in viewport |
| Energy | Warm + steady | Refreshing + crisp | Punchy + bold |

---

## How to pick

Open all three. Five-second gut reaction. Tell me:

- **"D1 — coral"** → I'll do 2-3 narrow variants (e.g. "what if the coral was muted", "more yellow accent", "tighter spacing")
- **"D2 — teal"** → same, narrow on teal
- **"D3 — gradient"** → same, narrow on gradient
- **"Hybrid — D1's warmth + D2's typography + D3's testimonial section"** → I'll merge
- **"None — too SMB, lean harder enterprise"** → I'll produce another 3 in the D direction but more "Notion enterprise"

Once a winner emerges, I author the implementation brief for the swarm to rebuild `marketing/` to match. Then we do the same exercise for the audit page (the in-product showpiece), then onboarding, then the rest.
