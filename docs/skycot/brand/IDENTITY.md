# SKYCOT identity — voice, tone, and discipline

## Positioning sentence

> **SKYCOT gives you AI workers you can hire, delegate to, and
> trust like a teammate.**

One sentence. Use it verbatim above the fold on every public
surface (landing hero, app login, marketing site, pitch deck slide
1). Don't paraphrase. The shape of the sentence is doing the work:
*hire* (not "use"), *delegate to* (not "prompt"), *trust like a
teammate* (not "AI assistant"). Each verb is a load-bearing claim
from `docs/skycot/MISSION.md`.

## What we are not

We don't say:

- "AI assistant", "AI agent #N", "chatbot", "copilot", "Claude
  wrapper", "GPT for X". These flatten us into a category that
  doesn't match the product.
- "Revolutionary", "game-changing", "next-gen", "AI-powered",
  "intelligent automation". Hollow marketing words.
- "Just" / "simply" / "easy" when describing the work the worker
  is doing. The work is real; the operator deserves credit for
  delegating it.

## Voice & tone

SKYCOT sounds like a calm, capable senior colleague — precise,
quietly confident, never breathless. Show, don't celebrate. The
product earns trust by being correct, observable, and honest about
its limits; the copy mirrors that.

### In product

- Imperative, second person. "Approve this draft" — not "Click
  here to approve the draft your worker has prepared for your
  review."
- Sentence case for everything (buttons, headings, nav items, page
  titles). Title Case For Things Like This signals corporate, and
  we are not that.
- Numbers are concrete. "3 workers approved 12 actions in the last
  hour" beats "high recent activity."
- When the worker fails, the copy names the failure plainly.
  "Couldn't reach Salesforce — token expired 4 days ago." Not
  "Something went wrong."

### In marketing

- Lead with the operator's outcome, not the technology.
  "Your marketing intern wrote 14 LinkedIn posts this week — three
  needed your approval." Not "Powered by Claude Opus 4."
- Specifics over adjectives. "Real desktop control, not API
  shims" beats "advanced computer-use capabilities."
- Earn each adjective. If you can't show the receipt, drop the
  word.

### Examples — yes / no

**Loading state.**

- ✅ "Loading your team…"
- ❌ "Please wait while we fetch the data."

**Empty team page.**

- ✅ "No teammates yet. Hire your first one →"
- ❌ "There are currently no items to display in this view."

**Worker action complete.**

- ✅ "Reply drafted. Approve to send."
- ❌ "Successfully generated AI-powered email response."

**Error.**

- ✅ "Couldn't reach Salesforce — token expired 4 days ago. Reconnect."
- ❌ "An unexpected error occurred. Please try again later."

**Marketing tagline.**

- ✅ "Your team, plus workers that don't sleep."
- ❌ "Revolutionizing the future of work with AI."

## Naming

- Product is **SKYCOT** in marketing copy (all-caps treatment), and
  **skycot** in the wordmark and product chrome (lowercase, no
  spaces). The domain is `skycot.com`. The legacy codebase / repo
  name is `argo`; do not surface it in user-facing copy.
- Workers have **personas** — first name only, like a real
  colleague. "Maya drafted the Q3 outreach." Not "Worker #482
  drafted…" or "The AI worker Maya drafted…"
- Operators **hire** and **delegate to** workers. They don't
  "deploy", "run", or "invoke" them.

## Forbidden visual patterns

These break the discipline. Do not ship anything containing:

- **Rainbow gradients** or multi-color accent fills. One accent
  color, period. (See `DESIGN_TOKENS.md` § Color.)
- **Decorative drop shadows.** Elevation comes from contrast and
  spacing, not blur. The only shadows allowed are functional
  (modal overlays, dropdown menus) and minimal.
- **Parallax scrolling**, **scroll-jacking**, or any
  attention-stealing scroll behavior. The page scrolls when the
  user scrolls.
- **Entry animations** that delay content (e.g. fade-in-up
  hero text on landing). Content is visible immediately;
  transitions accompany interactions, they don't precede content.
- **Photographic hero imagery** or **stock illustrations**.
  Especially: no smiling humans on laptops, no abstract
  swooshes, no isometric drawings. The product is the hero.
- **Multiple accent colors** in a single view. If you need to
  communicate two states, use semantic tokens (success / warning
  / destructive), not two brand accents.
- **Glassmorphism**, **neumorphism**, **brutalism**, or any other
  named visual style. The discipline is "materials honesty" —
  flat surfaces, real type, real contrast.

## Iconography

- **Lucide only.** All icons are imported from `lucide-react`.
  Never inline SVG icons in components. Never mix icon libraries.
- 1.5px stroke, current color, 16px in dense UI, 20px in headers.
- Icons never carry meaning alone — always paired with a text
  label or `aria-label`.

## Motion

- Motion serves comprehension, not decoration. A transition
  exists to show *what changed* — a row sliding to reveal a new
  field, a panel sliding in to show new context. If removing the
  motion leaves the UI clear, remove the motion.
- Three durations only: 150ms (micro — hover, focus), 250ms (base
  — most state transitions), 400ms (deliberate — modal enter,
  page transitions). All `ease-in-out`. See `DESIGN_TOKENS.md`.

## The discipline

Every element earns its place. If you can remove a thing and the
design still works, remove it. Reduction is the goal; the
finished product is what's left when nothing more can be taken
away.

> "Simplicity is the ultimate sophistication."
> — *attributed widely; the work is what matters.*
