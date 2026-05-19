# SKYCOT brand guidelines

**Version 1.0** · 2026-05-19 · Codified from 10 locked product designs · Built in Melbourne

This document is the single source of truth for every visible SKYCOT surface — marketing site, in-product UI, signup, onboarding, audit logs, and everything between. It supersedes the 2026-04 PDF brand spec on the points where the two diverge (notably: gradient color and ink color).

---

## 1 · Brand essence

SKYCOT is **"don't buy another tool. hire a teammate."**

We are the platform where customers hire AI workers and treat them like the colleagues they are. The brand language reflects this:

- **Warm, not cold** — paper backgrounds tinted lavender, ink tinted purple, gradients in pink/coral not slate
- **Confident, not bureaucratic** — Audiowide hero typography on every page makes every screen feel like it has something to say
- **Trustworthy, not generic** — cryptographic hash chains, SOC 2 / GDPR badges, and rich audit logging live alongside the warmth
- **Built in Melbourne, Australia** — never Sydney. Reference this proudly in footers.

**Brand register principle:** Every page wears the same recipe (Audiowide hero, purple→pink gradient, warm radial blobs). What scales by surface is the **size** of the Audiowide hero, not its presence. Marketing landing leads at 72px. Onboarding welcome leads at 96px. In-product surfaces drop to 36–44px. The voice is uniform; the volume scales.

---

## 2 · Logo system

### Brand mark

```
●  Width: 28–32px
   Border-radius: 6–7px
   Background: linear-gradient(135deg, var(--purple), var(--pink))
   Shadow (where bg permits): 0 4px 12px -2px rgba(139,92,246,0.30)
```

The mark is a square with the brand gradient. It always sits to the left of the wordmark, separated by 10–12px gap.

### Wordmark

The word `SKYCOT` set in **Audiowide regular (400)** with `letter-spacing: 0.08–0.10em`. Always uppercase (Audiowide has no lowercase letterforms). 16px on in-product surfaces, larger on marketing landing.

```css
.brand .name {
  font-family: 'Audiowide', sans-serif;
  font-size: 16px;
  font-weight: 400;
  letter-spacing: 0.10em;
}
```

### Clear-space

Minimum clear-space around the lockup (mark + wordmark) is equivalent to half the brand mark's width. Don't crowd it with nav links or chrome.

### Do / Don't

✓ Use the mark on light backgrounds with its native gradient
✓ Wordmark in Audiowide weight 400, never bolder, never lighter
✗ Don't recolor the mark to a single solid color
✗ Don't substitute another typeface for the wordmark
✗ Don't separate the mark and wordmark across the page

---

## 3 · Typography

Three families. No others.

| Use | Family | Weight | Notes |
|---|---|---|---|
| **Hero typography** (h1, page headings) | **Audiowide** | 400 | Single weight only. Letter-spacing 0 |
| **Wordmark + display moments** (eyebrow grad text, level numbers) | **Audiowide** | 400 | Letter-spacing 0.04–0.10em |
| **Body, UI, labels** | **Inter** | 300, 400, 500, 600, 700 | Letter-spacing -0.005em baseline |
| **Editorial display moments inside cards** | **Inter** | 700, 800, 900 | Used sparingly — only where Audiowide would overpower |
| **Mono data** (hashes, IDs, timestamps, status pills, kbd hints) | **JetBrains Mono** | 400, 500, 600 | Letter-spacing 0–0.04em |

### Hero scale by surface

The Audiowide hero is uniform in style; only the size changes:

| Surface type | h1 size | Examples |
|---|---|---|
| Marketing landing hero | 72px | "DON'T BUY ANOTHER TOOL." |
| Onboarding welcome | 96px | "BUILD YOUR AI WORKFORCE." |
| Auth signup / verify / signin | 56px | "HIRE YOUR FIRST AI TEAMMATE." |
| Workers / Skills / Worker detail | 44px | "Your roster" |
| Cohorts | 40px | "Browse cohorts by department" |
| Audit / Approvals / Settings | 36px | "Block #1,247 · chain head" |

### Hero gradient-text emphasis

Strong (`<strong>`) inside hero h1 always fills with the 3-color gradient:

```css
h1 strong {
  font-weight: 400;
  background: var(--gradient-3);
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent;
}
```

The non-emphasized parts of the h1 stay in `var(--ink)`. The emphasized parts pop in the brand gradient.

### Inter usage rules

- Body copy: **400 weight**, 15–17px on hero surfaces, 13–14px in-product
- Emphasis: **600** (rare), **700** (rarer)
- Editorial display when Audiowide would be too loud: **800 or 900** weight at 22–30px (used in worker-card stat numbers, etc.)
- Letter-spacing: `-0.005em` for body, tighter (`-0.025em`) on large display numerals

### JetBrains Mono usage

Reserved for data the user reads as machine-truth:
- SHA-256 hashes, block numbers, IDs
- Timestamps and SLA counters
- Keyboard shortcut hints (`⌘ K`, `↵`)
- Skill names (`zendesk.read`, `policy.tone`)
- Stat-row labels in uppercase (letter-spacing 0.1–0.12em)

Never use mono for body copy or hero typography.

---

## 4 · Color system

### Primary palette

```css
--purple:      #8B5CF6;  /* primary brand */
--purple-deep: #6d28d9;  /* hero gradient eyebrow text */
--pink:        #EC4899;  /* gradient partner + warm accents */
--coral:       #FF6B6B;  /* warm accents + 3-color gradient terminator */
--yellow:      #FCD34D;  /* warm accents + warning states */
--teal:        #14B8A6;  /* success states + warm-cool balance */
--cyan:        #0891B2;  /* used inside dark moments only */
```

### Soft tints (eyebrows, badges, subtle backgrounds)

```css
--soft-purple: #f3e8ff;
--soft-pink:   #fce7f3;
```

### Ink + surface

```css
--ink:    #1e1b4b;  /* primary text — PURPLE-TINTED, not neutral black */
--ink-2:  #3F3F46;  /* secondary text */
--ink-3:  #71717A;  /* tertiary text + labels */
--ink-4:  #A1A1AA;  /* placeholder + disabled */
--bg:     #fafaff;  /* page background — warm purple-tinted white */
--paper:  #F9FAFB;  /* card surfaces (slightly differs per file) */
--paper-2:#F3F4F6;  /* secondary cards / hover states */
--line:   #ece9f5;  /* hairlines — PURPLE-TINTED */
--line-2: #D1D5DB;  /* stronger borders */
```

### The signature gradient

```css
/* Primary 2-color gradient — for buttons, brand marks, accents */
--gradient: linear-gradient(135deg, var(--purple), var(--pink));

/* 3-color hero gradient — for h1 emphasis text fill */
--gradient-3: linear-gradient(135deg, var(--purple), var(--pink), var(--coral));
```

**Important:** SKYCOT's gradient is **purple → pink**, NOT the official 2026-04 brand spec's purple → cyan. The 2026-05-19 brand-consistency pass adopted the Canva-style warm palette throughout because the marketing landing diverged and the operator chose to keep that direction. The purple → cyan gradient from the spec is **not used** anywhere in v1.0 product surfaces.

### Semantic colors (status)

| State | BG tint (rgba) | Text |
|---|---|---|
| **OK / Succeeded / Active** | `rgba(20,184,166,0.10)` | `#0F766E` (deep teal) |
| **Pending / Warning / Approval needed** | `rgba(252,211,77,0.18)` | `#92400E` (deep amber) |
| **Refused / Critical / Danger** | `rgba(255,107,107,0.12)` | `#B91C1C` (deep coral) |
| **Selected / Highlighted** | `rgba(139,92,246,0.08)` | `var(--purple)` |
| **New / Just-released** | gradient (purple→pink) | white |

Status dots are tiny `6px` circles in the matching solid color, often with a 6–8px box-shadow glow for the live-now feeling.

---

## 5 · Background recipe (universal)

Every page applies the same body background — flat warm-white plus two radial blobs.

```css
body {
  background: var(--bg);  /* #fafaff */
  position: relative;
}
body::before {
  content: ""; position: fixed;
  top: -20%; right: -10%;
  width: 700px; height: 700px;
  background: radial-gradient(
    circle,
    rgba(139,92,246,0.15) 0%,
    transparent 70%
  );
  z-index: 0; pointer-events: none;
}
body::after {
  content: ""; position: fixed;
  bottom: -30%; left: -10%;
  width: 800px; height: 800px;
  background: radial-gradient(
    circle,
    rgba(236,72,153,0.10) 0%,
    transparent 70%
  );
  z-index: 0; pointer-events: none;
}
.app { position: relative; z-index: 1; }
```

This produces the soft "lavender wash top-right, soft pink wash bottom-left" feeling on every surface without being loud. The blobs are fixed so they don't scroll — they're ambient brand, not decoration.

---

## 6 · Component library

### Eyebrow pill (the "live indicator")

Used above every hero. Compact, mono uppercase, with an animated live-dot.

```css
.eyebrow {
  display: inline-flex; align-items: center; gap: 7px;
  padding: 8px 16px; border-radius: 999px;
  background: linear-gradient(135deg, var(--soft-purple), var(--soft-pink));
  font-family: 'JetBrains Mono', monospace;
  font-size: 11–13px;
  font-weight: 700;
  letter-spacing: 0.20em;
  text-transform: uppercase;
  color: var(--purple-deep);
}
.eyebrow .pulse {
  width: 8px; height: 8px; border-radius: 50%;
  background: var(--pink);
  box-shadow: 0 0 0 4px rgba(236,72,153,0.25);
  animation: pulse 2s ease-in-out infinite;
}
@keyframes pulse {
  50% { box-shadow: 0 0 0 8px rgba(236,72,153,0.15); }
}
```

### Primary CTA — "Hire" / "Get started"

```css
.btn-primary {
  background: linear-gradient(135deg, var(--purple), var(--pink));
  color: white;
  padding: 14–17px 24–30px;
  border-radius: 999px;  /* fully rounded on marketing */
  /* OR */
  border-radius: 10–12px; /* in-product */
  font-weight: 700;
  font-size: 14–16px;
  box-shadow: 0 6–12px 20–28px -4px rgba(139,92,246,0.40–0.50);
  transition: transform 0.15s, box-shadow 0.15s;
}
.btn-primary:hover {
  transform: translateY(-2px);
  box-shadow: 0 12–16px 28–32px -8px rgba(139,92,246,0.55–0.60);
}
```

### Secondary CTA

```css
.btn-secondary {
  background: white;
  color: var(--ink);
  padding: 14–17px 22–26px;
  border: 1px solid var(--line);
  border-radius: 999px or 10px;
  font-weight: 600;
}
```

### Worker / portrait avatars (the gradient-square pattern)

```css
.av {
  width: 32–48px; height: 32–48px;
  border-radius: 8–12px;
  display: grid; place-items: center;
  color: white;
  font-family: 'Audiowide', sans-serif;
  font-size: 10–14px;
  letter-spacing: 0.04em;
  background: var(--gradient);
}
/* Per-worker variants */
.av.s { background: linear-gradient(135deg, var(--pink),   var(--coral)); }   /* Sara */
.av.p { background: linear-gradient(135deg, var(--purple), var(--pink));  }   /* Priya */
.av.n { background: linear-gradient(135deg, var(--teal),   var(--cyan));  }   /* Nori */
.av.m { background: linear-gradient(135deg, var(--coral),  var(--yellow));}   /* Mariah */
```

The avatar gradient palette per worker is the *visual fingerprint* of that worker. Reuse the same gradient mapping everywhere a worker appears.

### Card surfaces

Cards float on the warm background — they're white with a subtle line border and a soft shadow:

```css
.card {
  background: white;
  border: 1–2px solid var(--line);
  border-radius: 12–18px;
  padding: 16–32px;
  box-shadow:
    0 1px 2px rgba(0,0,0,0.03),
    0 8–32px 20–60px -8px rgba(139,92,246,0.10–0.20);
  transition: transform 0.15s, border-color 0.15s, box-shadow 0.15s;
}
.card:hover {
  transform: translateY(-2–6px);
  border-color: var(--line-2);
  box-shadow: 0 16–48px 32–64px -16px rgba(139,92,246,0.15–0.30);
}
```

Brand-loud cards (Workers, Cohorts, marketing feature cards) get a 4px top accent-stripe in the brand gradient.

### Status pill

```css
.pill {
  display: inline-flex; align-items: center; gap: 5px;
  padding: 3–4px 9–11px; border-radius: 999px;
  font-size: 11–12px;
  font-weight: 700;
}
.pill .dot { width: 5–7px; height: 5–7px; border-radius: 50%; }
.pill.ok   { background: rgba(20,184,166,0.10); color: #0F766E; }
.pill.warn { background: rgba(252,211,77,0.18); color: #92400E; }
.pill.bad  { background: rgba(255,107,107,0.12); color: #B91C1C; }
.pill.ok .dot   { background: var(--teal);   box-shadow: 0 0 6px var(--teal); }
.pill.warn .dot { background: var(--yellow); box-shadow: 0 0 6px var(--yellow); }
.pill.bad .dot  { background: var(--coral);  box-shadow: 0 0 6px var(--coral); }
```

### Keyboard kbd hint

```css
.kbd {
  font-family: 'JetBrains Mono', monospace;
  font-size: 10.5–12px;
  padding: 2–5px 6–9px;
  border-radius: 4–6px;
  background: var(--paper-2);  /* or rgba(255,255,255,0.25) inside a gradient button */
  color: var(--ink-2);
  font-weight: 700;
  letter-spacing: 0.04em;
}
```

---

## 7 · Avatar / portrait system

Five seeded SKYCOT workers have fixed avatar gradients used everywhere they appear:

| Worker | Initials | Gradient |
|---|---|---|
| **Jamal** (Support) | `JM` | purple → cyan (the primary brand gradient — Jamal is the default/L1 worker) |
| **Sara** (Invoice ops) | `SL` | pink → coral |
| **Priya** (Recruiting) | `PR` | purple → pink |
| **Nori** (Quote ops) | `NM` | teal → cyan |
| **Mariah** (Sales ops) | `MA` | coral → yellow |

Specialty cohorts get gradients from the adjacent palette (AR Collector: yellow → teal; CS Responder: cyan → purple; etc.).

The avatar shape is always a **rounded square** (8–12px radius), never circular. Worker initials are set in **Audiowide regular** with a subtle letter-spacing.

---

## 8 · Voice + copywriting

### Hero copy patterns

The hero is always a complete thought — a sentence or a confident statement. Periods belong. The emphasized word(s) are in gradient text.

✓ "DON'T BUY ANOTHER TOOL. HIRE A **TEAMMATE.** (SERIOUSLY.)"
✓ "BUILD YOUR **AI WORKFORCE.**"
✓ "Hire your first **AI teammate**."
✓ "Meet **your first teammate**."

✗ "AI for your business" (too vague)
✗ "The future of work is here" (cliché)
✗ "Welcome to SKYCOT" (lazy)

### Tone

- **Conversational + confident.** Talk to the operator like a peer, not a customer.
- **Specific numbers over generic claims.** "94% resolution rate" beats "high accuracy."
- **Worker names in actions.** "Nori flagged a 32% discount" beats "the worker detected an anomaly."
- **Time-bound when possible.** "In four minutes, you'll have hired your first teammate."
- **Plain language.** Per [`user_plain_language_no_jargon.md`](.claude/projects/-Users-operator-code-argo/memory/user_plain_language_no_jargon.md): expand every abbreviation on first use; every dev term gets a one-line plain-English summary.

### Microcopy patterns

| Where | Pattern |
|---|---|
| CTAs | Verb-first, action-oriented. "Hire your first worker", "Send sign-in link", "Approve & send" |
| Eyebrow labels | Status + context, mono uppercase. "★ FEATURED · NEW THIS MONTH", "LIVE · 17 TEAMS ONBOARDING NOW" |
| Trust footers | Compliance badges in mono uppercase. "NO CREDIT CARD · SOC 2 TYPE II · GDPR READY" |
| Status pills | Single word or two. "Active", "Pending you", "Refused" |
| Refusal reasoning | Specific + explainable. "Refused duplicate invoice INV-2024-08739 — policy: no_duplicate_invoice_30d" |

---

## 9 · Imagery + illustration

The marketing landing uses a friendly **MAYA character** — a chat bubble with a smile — and floating decorative blobs. As of v1.0 this lives only on the marketing landing; other surfaces use the avatar gradients + radial-blob backgrounds as their brand-imagery anchor.

If illustrations are extended later, follow these rules:
- Geometric + friendly, never literal
- Use the Canva palette (purple/pink/coral/yellow/teal)
- Drop shadows are soft, never harsh
- Stroke weight 2px; rounded line ends
- One main character per illustration moment

---

## 10 · Application reference — the 10 locked surfaces

Every locked page is a worked example of these guidelines.

| Surface | Locked file | Register notes |
|---|---|---|
| **Marketing landing** | `marketing-landing/option-D3-with-audiowide.html` | 72px hero, MAYA illustration, dark testimonial moment |
| **Auth (signup / verify / signin)** | `auth-pages/option-A-editorial.html` + `page-verify.html` + `page-signin.html` | 56px hero, centered, magic-link |
| **Onboarding wizard (5 steps)** | `onboarding-wizard/option-F1-editorial-center.html` + steps 2–5 | 64–96px hero, A-style top bar with pip strip |
| **Workers** | `workers-page/option-C2-holographic-showcase.html` | 44px hero, trading-card grid, banner gradients |
| **Worker detail** | `worker-detail/option-C2-personality.html` | 44px hero (worker name), trait chips, "great at / pull in" framing |
| **Approvals** | `approvals-inbox/option-E-ops-dashboard.html` | 36px hero, KPI strip, severity-tier sections |
| **Audit** | `audit-page/option-B3-enterprise.html` | 36px hero, block-explorer pattern, pop-out modal |
| **Skills** | `skills-page/option-A1-prominent-search.html` | 44px hero, hero search bar, card grid |
| **Cohorts** | `cohorts-page/option-D3-magazine-tiles.html` | 40px hero, 6 dept tabs, magazine tile layout |
| **Settings** | `settings-page/option-D-top-tabs.html` | 36px hero, horizontal top tabs |

---

## 11 · Do's and Don'ts

✓ **Do** apply the full body background recipe on every page (radial blobs + warm white)
✓ **Do** use Audiowide for h1 + wordmark on every surface
✓ **Do** scale the Audiowide hero appropriately for the surface
✓ **Do** use `--gradient` (purple→pink) for primary CTAs and brand marks
✓ **Do** use `--gradient-3` (purple→pink→coral) for hero `<strong>` text fill
✓ **Do** use Inter for body, JetBrains Mono for technical data
✓ **Do** use the worker avatar gradient palette consistently across surfaces
✓ **Do** include status dots with box-shadow glow on live/active states

✗ **Don't** use the purple → cyan gradient anywhere in v1.0 (the 2026-04 brand spec is superseded on this point)
✗ **Don't** use `#18181B` for ink — use `#1e1b4b` (purple-tinted)
✗ **Don't** use pure-white `#FFFFFF` for the body background — use `#fafaff` (warm-tinted)
✗ **Don't** use Audiowide for body copy, labels, or anything below h1 — it's a display face
✗ **Don't** apply the brand gradient on top of dark gradients (avoid visual mud)
✗ **Don't** scale the Audiowide hero below 28px on any surface

---

## 12 · Implementation note for engineers

When building actual product code in `marketing/` or `web/`:

1. Copy the CSS variables block from `option-D3-with-audiowide.html` into your global stylesheet
2. Load `Audiowide`, `Inter`, and `JetBrains Mono` from Google Fonts in one combined `link` tag
3. Apply the `body::before` + `body::after` blobs at the root layout level so every route inherits them
4. Build atoms (eyebrow, btn-primary, status-pill, avatar, kbd) as reusable components
5. Reference the locked HTML mockup for each surface as the visual spec — pixel-match the layout, then componentize

---

## Changelog

- **2026-05-19 — v1.0**: Brand guidelines authored from the 10 locked product designs. Supersedes the 2026-04 PDF brand spec on the points noted (gradient color, ink color).

**Maintainer:** Mark Hallam · `founder@example.com` · Melbourne, Australia
