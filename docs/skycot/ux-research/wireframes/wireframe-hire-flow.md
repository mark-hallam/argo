# Wireframe — Hire Flow

The first 10 minutes of the product. Conversational onboarding
driven by Genesis-lite. Turns "I want help with X" into a hired
AI teammate with a charter, a persona, and a working approval
flow. The conversion moment for mission claim #3 ("the platform
builds workers").

## Purpose

Walk a fresh Delegator from "I have a problem to delegate" to
"I have a teammate handling it" in ≤ 10 minutes with zero
configuration files, zero jargon, and three clarifying
questions at most.

**Serves:**
- Persona: [delegator.md](../delegator.md) (primary)
- Journey: [journey-hire-a-teammate.md](../journey-hire-a-teammate.md) (canonical)

**Implementation primitives:**
- [Card](../../../../web/components/ui/Card.tsx),
  [Avatar](../../../../web/components/ui/Avatar.tsx),
  [Button](../../../../web/components/ui/Button.tsx)
- NEW: `ChatBubble`, `ProgressDots`, `ExampleMessagePreview`,
  `OAuthConnectorRow` — see "Implementation notes" below.

## Information architecture

- URL: `/hire` (entry) → `/hire/step/<N>` (per step) →
  `/hire/done/<workerId>` (completion).
- Nav placement: triggered from "+ Hire teammate" on the
  approvals inbox header. NOT a permanent nav item — the
  flow is one-shot, not recurring.
- Browser back works between steps (preserves state).
- Closing the tab mid-flow saves a draft; returning resumes
  at the last completed step.
- Final step routes to the new worker's profile
  (`/workers/<workerId>`).

## Layout — full-flow overview

```
+----------------------------------------------------------------+
|  (1) MODAL OR FULL-PAGE — see "shell" below                    |
|                                                                |
|  ● ● ● ○ ○ ○ ○   (2) PROGRESS DOTS — current step indicator    |
|                                                                |
|  ┌──────────────────────────────────────────────────────────┐ |
|  │                                                           │ |
|  │   (3) STEP CONTENT — one of:                             │ |
|  │                                                           │ |
|  │     • Step 1: Land                                       │ |
|  │     • Step 2: Listen (clarifying)                        │ |
|  │     • Step 3: Drafting… (transition)                     │ |
|  │     • Step 4: Introduce                                  │ |
|  │     • Step 5: Connect                                    │ |
|  │     • Step 6: First action (handoff to inbox)            │ |
|  │     • Step 7: Day-1 wrap (notification, not a screen)    │ |
|  │                                                           │ |
|  └──────────────────────────────────────────────────────────┘ |
|                                                                |
|  (4) [ < Back ]                              [ Continue → ]    |
|                                                                |
+----------------------------------------------------------------+
```

## Shell

The hire flow is a **full-page route**, not a modal. Reasoning:

- The flow is identity-shaping (this is the moment a person
  becomes a "delegator with an AI teammate"). It deserves the
  whole screen, not a corner of one.
- It contains free-text input + multi-line previews + OAuth
  popups. Modal interactions get squeezed.
- Browser back/forward and bookmarkable URLs help recovery.

The page hides the side rail. Only the SKYCOT brand square
shows top-left (clickable, returns to inbox with confirm
dialog if mid-flow).

## Annotations

**(1)** Shell. Full viewport. Max content width `max-w-2xl`,
centered. Vertical padding `py-12 md:py-16`. Brand square
top-left, escape-X top-right (with confirm if mid-flow has
unsaved state).

**(2)** Progress dots. 7 dots in a horizontal row.
- Filled `bg-[var(--primary)]` for completed steps.
- Filled `bg-[var(--primary)]/40` for current.
- Empty `bg-[var(--muted)]` for upcoming.
- Tooltips on hover show step name.
- Not clickable forward; clickable backward (jumps to
  previously-completed step, preserving its state).

**(3)** Step content. See per-step blocks below.

**(4)** Footer nav. Back goes to previous step. Continue
goes to next step. Both follow standard Button variants
(Back: ghost; Continue: primary). Continue is disabled
until the step's required fields are filled.

---

## Step 1 — Land

```
+----------------------------------------------------------------+
|  ● ○ ○ ○ ○ ○ ○                                                 |
|                                                                |
|       What do you need help with?  (1)                         |
|                                                                |
|       Tell me in your own words. Anything you'd love           |
|       to stop doing yourself.                                  |
|                                                                |
|       +------------------------------------------------+      |
|       | I'm drowning in marketplace replies.           | (2)  |
|       | I sell hockey gear on Facebook Marketplace,    |      |
|       | Gumtree, and eBay. I get 30 messages a day     |      |
|       | and half are 'is this still available'.        |      |
|       +------------------------------------------------+      |
|                                                                |
|       (3) [ ? Need some examples to start? ]                   |
|                                                                |
|  [ ← ]                                  [ Continue → ]  (4)    |
+----------------------------------------------------------------+
```

**(1)** Headline. `<h1>` text `"What do you need help with?"`
`text-3xl font-semibold tracking-tight`.

**Sub-line** below headline. `text-[var(--muted-foreground)]
text-sm`. Strings: `"Tell me in your own words. Anything
you'd love to stop doing yourself."`.

**(2)** Free-text textarea. Min height `min-h-[160px]`.
Auto-grows to content (max `max-h-[400px]` then scrolls).
Placeholder strings (cycle 3, picked at random on mount):
- `"e.g. 'I keep getting calendar invites I have to triage…'"`
- `"e.g. 'My inbox is full of "is this in stock" questions…'"`
- `"e.g. 'I need someone to qualify inbound leads…'"`

Submit on `Cmd/Ctrl + Enter` advances.

**(3)** "Need some examples?" button. Ghost variant.
Tapping opens a popover with 5 example role-descriptions,
each one a single sentence the user can click to fill the
textarea. Examples drawn from shipped verticals:
- Marketplace seller
- Recruiter screening candidates
- Customer support tier-1
- Sales SDR outreach
- Tax-prep document intake

**(4)** Continue button. Disabled until textarea has > 15
characters. Otherwise primary size `lg`.

**State variants:**
- **Empty (initial):** as drawn above.
- **Examples popover open:** 5 example rows over a soft
  backdrop. Tapping outside closes.
- **Submitting:** Continue button enters loading state for
  ~500ms while the input is sent to Genesis-lite, then
  routes to Step 2.

---

## Step 2 — Listen (clarifying questions)

```
+----------------------------------------------------------------+
|  ● ● ○ ○ ○ ○ ○                                                 |
|                                                                |
|       A few quick questions to set them up right.  (1)         |
|                                                                |
|  ┌────────────────────────────────────────────────────────┐   |
|  │  1.  Should they reply on your behalf,                 │   |
|  │      or only draft replies for you to send?      (2)   │   |
|  │                                                         │   |
|  │      ( ) Draft for me to send                          │   |
|  │      (•) Reply directly, ask for big calls             │   |
|  │      ( ) I'll decide later                             │   |
|  └────────────────────────────────────────────────────────┘   |
|                                                                |
|  ┌────────────────────────────────────────────────────────┐   |
|  │  2.  Are there things they should never do without     │   |
|  │      your okay — like agreeing to a price below your   │   |
|  │      floor?                                       (3)  │   |
|  │                                                         │   |
|  │      +-------------------------------------+          │   |
|  │      | Never go below $50 without checking |          │   |
|  │      +-------------------------------------+          │   |
|  └────────────────────────────────────────────────────────┘   |
|                                                                |
|  ┌────────────────────────────────────────────────────────┐   |
|  │  3.  What name should they use with your buyers?  (4)  │   |
|  │                                                         │   |
|  │      +------------+                                    │   |
|  │      | Mariah     |   ↻ suggest another                │   |
|  │      +------------+                                    │   |
|  └────────────────────────────────────────────────────────┘   |
|                                                                |
|  [ ← Back ]                            [ Continue → ]  (5)     |
+----------------------------------------------------------------+
```

**(1)** Headline. Strings: `"A few quick questions to set
them up right."` `text-2xl font-semibold`.

**(2)–(4)** Three question cards. Each card has:
- A numbered question heading.
- An answer surface that varies by question type:
  - Single-select radio group (Q1 here).
  - Free-text input (Q2).
  - Free-text input with a "suggest another" link (Q3).

The questions are NOT fixed. Genesis-lite generates them
from the Step 1 input and the matched vertical's
`[charter_questions]` table. Max 3 questions ever. The
shape stays the same; the question text and answer types
vary.

**Q1 default answer types:**
- Single-select radio (autonomy preference).
- Single-select radio (channel preference).
- Yes/no toggle (require approval for new contacts).

**(5)** Continue. Enabled when all questions have a
non-empty answer (or are skippable). Tooltip on disabled:
`"Answer the questions above to continue"`.

**State variants:**
- **Loading clarifying questions** (transitioning from
  Step 1): question cards show skeleton placeholders;
  Continue disabled.
- **Mid-fill:** as drawn.
- **All filled:** Continue active.

---

## Step 3 — Drafting (transition)

A 3-5 second wait state. Genesis-lite is generating the
worker draft (persona, charter, duties, example replies)
in the background. The user sees:

```
+----------------------------------------------------------------+
|  ● ● ● ○ ○ ○ ○                                                 |
|                                                                |
|              (Drafting an AI teammate for you…)                |
|                                                                |
|              [ Mariah                              ]           |
|              [ — building her persona               ] (1)      |
|              [ — drafting her first-week routine    ]          |
|              [ — picking a few opening examples     ]          |
|                                                                |
|              ⏳ ⏳ ⏳                                            |
|                                                                |
+----------------------------------------------------------------+
```

**(1)** Progress checklist. 3-4 items, each fading in over
~800ms, each check-marking when complete. Pure visual
candy — Genesis-lite is running but doesn't actually
checkpoint at these granularities; the UI fakes the
appearance of multi-stage progress for psychological
acceptance.

If Genesis-lite takes > 8 seconds, show a "still working…"
hint. If > 30 seconds, show a "this is taking longer than
usual" hint with a retry / contact-support escape route.

No back/continue buttons — auto-advances when Genesis
returns.

---

## Step 4 — Introduce

The load-bearing moment. The user sees the worker as a
person for the first time, with 3 concrete example
messages.

```
+----------------------------------------------------------------+
|  ● ● ● ● ○ ○ ○                                                 |
|                                                                |
|       Meet your new teammate.  (1)                             |
|                                                                |
|  ┌─────────────────────────────────────────────────────────┐  |
|  │  ╭───╮                                                   │  |
|  │  │ M │  Mariah                              [✎ Adjust]  │  |
|  │  ╰───╯  Marketplace assistant                            │  |
|  │                                                           │  |
|  │  She helps you manage your marketplace stores. She       │  |
|  │  reads inbound messages, drafts replies in your voice,   │  |
|  │  and asks before agreeing to anything below $50.    (2) │  |
|  │                                                           │  |
|  │  ─────────  Her first week                ─────────  (3)│  |
|  │                                                           │  |
|  │   📨  Read inbound messages every 30 min                 │  |
|  │   ✍   Draft replies to "is this available?"             │  |
|  │   💰  Flag any counter-offer below 90% of asking         │  |
|  │   📊  Evening price-check on listings > 7 days old       │  |
|  │                                                           │  |
|  └─────────────────────────────────────────────────────────┘  |
|                                                                |
|       Three example replies she'd send today:  (4)             |
|                                                                |
|  ┌─────────────────────────────────────────────────────────┐  |
|  │  💬  Buyer: "Is this still available?"                   │  |
|  │      Mariah → "Yes! Still available at $90. Want me      │  |
|  │      to hold it for pickup this weekend?"                │  |
|  └─────────────────────────────────────────────────────────┘  |
|                                                                |
|  ┌─────────────────────────────────────────────────────────┐  |
|  │  💬  Buyer: "Can you do $80?"                            │  |
|  │      Mariah → "I need to check with Sam on counter-      │  |
|  │      offers. I'll get back to you within the hour."      │  |
|  └─────────────────────────────────────────────────────────┘  |
|                                                                |
|  ┌─────────────────────────────────────────────────────────┐  |
|  │  💬  Buyer: "Where can you ship to?"                     │  |
|  │      Mariah → "Local pickup in Auckland is free, or      │  |
|  │      courier across NZ for $18 standard."                │  |
|  └─────────────────────────────────────────────────────────┘  |
|                                                                |
|  Sound right?                                                  |
|  [ ✎ Adjust Mariah ]  [ Hire Mariah → ]  (5)                   |
+----------------------------------------------------------------+
```

**(1)** Headline. Strings: `"Meet your new teammate."`
`text-2xl font-semibold`.

**(2)** Worker profile card. Components:
- Avatar (size lg, generated initials by default; image
  upload deferred to v2).
- Name (editable inline).
- Role tagline (Genesis-generated, editable inline).
- Charter — 2-3 sentence summary, click to edit (opens
  a small textarea inline).
- "Adjust" button (top-right) opens a full charter-editor
  drawer (out of scope here — see Adjust path).

**(3)** Duties preview. 3-5 cron'd / event-triggered
actions Mariah will perform out of the gate. Icon + plain-
English description. Tap to edit (toggle on/off or modify
schedule, opens drawer).

**(4)** Example messages. Three card-sized previews. Each
shows:
- An inbound message from a synthetic buyer.
- Mariah's drafted response in the user's voice profile.

The voice profile inherits from the user's stated tone /
formality / lexicon (collected later in onboarding or
pulled from their prior message history if connected).

**(5)** Action row. Two buttons:
- Adjust Mariah (ghost): opens a single free-text field
  asking `"What would you change about how Mariah works?"`.
  Genesis-lite re-runs. New examples. New profile. Loop
  until "Hire Mariah".
- Hire Mariah (primary, lg): commits the worker. Routes
  to Step 5.

**State variants:**
- **Default:** as drawn.
- **Adjust panel open:** a drawer slides in from the right
  with the free-text field. Re-drafting reuses Step 3's
  transition state.
- **Hire submitting:** button enters loading state for
  ~1.5s while the worker is created server-side; routes
  to Step 5 on success.

---

## Step 5 — Connect

```
+----------------------------------------------------------------+
|  ● ● ● ● ● ○ ○                                                 |
|                                                                |
|       Mariah needs access to a few things.  (1)                |
|       You can skip — she'll work with whatever you have.       |
|                                                                |
|  ┌─────────────────────────────────────────────────────────┐  |
|  │  📘 Facebook Marketplace          ○ Not connected       │  |
|  │     Read + reply on your listings.        [ Connect ]   │  |
|  └─────────────────────────────────────────────────────────┘  |
|                                                                |
|  ┌─────────────────────────────────────────────────────────┐  |
|  │  🛒 Gumtree                       ○ Not connected  (2)  │  |
|  │     Read + reply on your listings.        [ Connect ]   │  |
|  └─────────────────────────────────────────────────────────┘  |
|                                                                |
|  ┌─────────────────────────────────────────────────────────┐  |
|  │  📦 eBay                          ✓ Connected (now)     │  |
|  │     Read + reply on your listings.    [ Disconnect ]    │  |
|  └─────────────────────────────────────────────────────────┘  |
|                                                                |
|       Reach you for approvals via:                             |
|                                                                |
|  [ Slack ]  [ iMessage ]  [ Email ]  [ SMS ]  ... (3)          |
|       ✓ Slack selected. Others available later.                |
|                                                                |
|  [ ← Back ]                  [ Skip for now ]  [ Continue → ]  |
+----------------------------------------------------------------+
```

**(1)** Headline. Strings: `"Mariah needs access to a few
things."`. Sub-line: `"You can skip — she'll work with
whatever you have."`.

**(2)** OAuth connector row. Each row shows:
- Logo + name.
- Status: `"Not connected"` / `"Connected"` / `"Re-auth needed"`.
- Description (one line).
- Action button: `[ Connect ]` / `[ Disconnect ]` / `[ Re-auth ]`.

Connect launches the standard OAuth popup for that
connector (existing argo-connector flow). On success, the
row flips to Connected state.

For connectors without OAuth (e.g. Facebook Marketplace —
which is computer-use-driven), Connect launches a different
flow: "Log in on this device, we'll capture the session
for Mariah." Out of scope for this wireframe's depth —
referenced briefly with an asterisk and a tooltip.

**(3)** Approval channel selector. Single-select for the
PRIMARY channel; multi-select for fallback channels. v1
ships single-select. The user picks where they want to be
asked for approvals (defaults to whichever channel they're
already on — detected via cookie / prior gateway auth).

**Footer:**
- Back: returns to Step 4.
- Skip for now: routes to Step 6 (hire complete; Mariah is
  inert until at least one connector + one channel are
  set, but the worker exists).
- Continue: routes to Step 6.

**State variants:**
- **Initial (no connections):** all rows show "Not
  connected". Channel row shows the auto-detected channel
  pre-selected.
- **Mid-connect (popup open):** row enters a "connecting…"
  loading state.
- **Re-auth needed:** previously-connected, token expired.
  Red dot on the status; button shows "Re-auth".
- **Connect failed:** error state — red border on the row,
  error message below, action button shows "Try again".

---

## Step 6 — First action (handoff)

Not a step you spend time on; the user lands on the
approvals inbox with a celebratory banner.

```
+----------------------------------------------------------------+
|  (back at the approvals inbox layout — see                    |
|   wireframe-approvals-inbox.md)                                |
|                                                                |
|  ┌─────────────────────────────────────────────────────────┐  |
|  │  🎉  Mariah is hired. She'll start working in the       │  |
|  │      background. The first time she has an opinion,     │  |
|  │      you'll see it here.                                │  |
|  │                          [ Open Mariah's profile → ]    │  |
|  └─────────────────────────────────────────────────────────┘  |
|                                                                |
|  (rest of approvals inbox as usual — likely empty or          |
|   has cards from other workers if any)                         |
+----------------------------------------------------------------+
```

The banner stays visible for the session and dismisses on
first interaction. The "Open Mariah's profile" button
routes to `/workers/<workerId>`.

If Mariah produces her first approval request within ~10
minutes of hire, push a notification (via the user's
chosen channel from Step 5). If not, the inbox just sits
and waits.

---

## Step 7 — Day-1 wrap (notification)

NOT a screen. A notification sent ~24 hours after hire
via the user's chosen channel. See
[journey-hire-a-teammate.md §"Step 7 — Day-1 wrap"](../journey-hire-a-teammate.md)
for content.

Surface implication: the day-1-wrap content must be
generable by a worker's audit-feed query +
day-summary skill. No bespoke UI here.

---

## States (cross-step)

### Resume-from-draft

If user closes the tab between Step 2 and Step 5, returning
to `/hire` re-loads at the last completed step with their
inputs preserved. Server stores the draft against the
session. Draft expires after 7 days.

### Network failure mid-flow

If Genesis-lite call fails in Step 3, surface an error
state with retry. The draft data from Steps 1-2 is
preserved.

### Permission-denied

If the user is on a tenant that has reached its worker-
hire quota, surface a quota-exceeded message before Step
1 with a CTA to upgrade or contact the operator. Out of
v1 — initial deployments don't enforce per-tenant quotas.

### Mobile layout

The hire flow works on mobile. All adjustments:
- Step 1 textarea is full-width.
- Step 2 question cards stack 1-up (already do at
  `max-w-2xl`).
- Step 4 example-message cards stack 1-up; the worker
  profile card sits above them.
- Step 5 connector rows are full-width; channel selector
  becomes a vertical list of toggles.
- Progress dots stay at the top.
- Continue/Back buttons fixed to the bottom of the
  viewport (above the safe-area inset) so they're always
  reachable.

---

## Interactions + flows

### Standard happy path

```
/hire
  → Land (Step 1, free-text input)
  → Listen (Step 2, 3 questions)
  → Drafting transition (Step 3)
  → Introduce (Step 4, example messages)
  → [Hire button]
  → Connect (Step 5, OAuth)
  → [Continue or Skip]
  → /workers/<workerId> with celebration banner (Step 6)
  → 24h later: Day-1 wrap notification (Step 7)
```

Median expected time: 6-8 minutes from `/hire` open to
celebration banner.

### Adjust path

```
Step 4
  → [Adjust Mariah button]
  → free-text "What would you change?"
  → Re-runs Steps 3 + 4 with the new input
  → Loop until [Hire Mariah]
```

Adjust counter: if the user hits Adjust 3+ times, surface
a fallback: `"Want to start with a specific template?"` and
offer the vertical-list picker. The conversational path is
default; templates are escape hatch.

### Adjust drawer (charter editor)

Tapping the "Adjust" button at the top-right of the worker
card in Step 4 opens a drawer (Radix Dialog from the right):

```
+-----------------------------+
| Adjust how Mariah works     |
|                             |
| Charter (what she does):    |
| +--------------------------+|
| | She helps you manage...  ||
| +--------------------------+|
|                             |
| Duties (her first week):    |
| □ Read inbound messages...  |
| □ Draft replies...          |
| □ Flag counter-offers...    |
| □ Evening price-check       |
| [+ Add another duty]        |
|                             |
| Voice (how she sounds):     |
| Tone: [warm/neutral/formal] |
| Formality: [—o——]           |
| Signature phrases:          |
| +--------------------------+|
| | "Cheers!"                ||
| +--------------------------+|
|                             |
| [ Cancel ]  [ Save changes ]|
+-----------------------------+
```

Saves return to Step 4 with regenerated example messages
based on the edits.

---

## Empty + error copy (exact strings, locked)

| Moment | Copy |
|---|---|
| Step 1 placeholder (rotating) | `"e.g. 'I keep getting calendar invites I have to triage…'"` / `"e.g. 'My inbox is full of \"is this in stock\" questions…'"` / `"e.g. 'I need someone to qualify inbound leads…'"` |
| Step 2 disabled-continue tooltip | `"Answer the questions above to continue"` |
| Step 3 long-wait hint (>8s) | `"Still working — usually only takes a few seconds."` |
| Step 3 very-long-wait (>30s) | `"This is taking longer than expected. [ Try again ]"` |
| Step 4 Adjust-loop-3 fallback | `"Want to start with a specific template?"` |
| Step 5 skip button | `"Skip for now"` |
| Step 5 skip-confirm dialog (if skip) | `"Mariah will be hired but won't have anything to work on yet. You can connect tools later from her profile. Continue?"` |
| Step 6 celebration banner | `"Mariah is hired. She'll start working in the background. The first time she has an opinion, you'll see it here."` |
| Quota exceeded | `"You've reached your workspace's teammate limit. Contact your operator to add more."` |

---

## Implementation notes for Track E

This is the most complex surface in the v1 set. Track E
should slice it as follows:

- **E2-a: Hire flow — shell + routing.** Sets up `/hire`,
  `/hire/step/<N>`, progress dots, back/continue, draft
  persistence. No step content yet.
- **E2-b: Hire flow — Step 1 (Land).** Free-text input,
  placeholder cycling, examples popover, submit → API call
  to start a hire session.
- **E2-c: Hire flow — Step 2 (Listen).** Renders the
  Genesis-lite-generated questions. Form state for 3
  answers.
- **E2-d: Hire flow — Step 3 (Drafting transition).** The
  fake-progress checklist with timing.
- **E2-e: Hire flow — Step 4 (Introduce).** Worker profile
  card + example message rendering. Adjust button → drawer.
- **E2-f: Hire flow — Adjust drawer.** Charter editor +
  duty toggles + voice profile editor. Save → re-run
  Step 3.
- **E2-g: Hire flow — Step 5 (Connect).** OAuth row
  components. Channel selector.
- **E2-h: Hire flow — Step 6 (Celebration banner).** New
  banner component in the approvals-inbox header.
- **E2-i: Hire flow — Day-1 wrap notification.** Worker
  duty that fires at hire+24h and produces a summary.
  Backend brief, not a UI brief.

Total: ~10 briefs. Most are ≤ 250 LOC. Step 4 (Introduce)
is the largest at ~350 LOC because it composes 4 sub-
components.

New primitives to add to the design system:
- `ProgressDots`: 7-position indicator. Tooltips on hover.
- `ChatBubble`: for the example-message preview format
  (inbound + outbound speech bubble).
- `OAuthConnectorRow`: status + action wrapper. Re-usable
  for worker profile's connectors tab too.
- `ChannelToggleRow`: similar to OAuthConnectorRow but for
  approval-channel selection.

---

## Open questions

`(needs validation)`

- Is full-page right, or should this be a modal over the
  approvals inbox?
- Is the free-text Step 1 input correct, or do users
  default to choosing from a list of canned roles?
- Are the example messages in Step 4 worth the LLM cost
  per hire? If yes, are 3 the right number?
- Should the Adjust drawer expose voice-profile editing,
  or is that overwhelming for first-hire?
- Is Step 5 (Connect) too soon — should we defer to after
  the first action so users can see the value first?
- Day-1 wrap: notification on first day vs first 24-hour
  email vs first-Sunday-summary cadence.

Plan to validate by:
1. Shipping the flow at v1, watching the first 50
   delegators end-to-end.
2. Time-to-hire telemetry: median, p90, p99.
3. Adjust-rate telemetry: how often, when, does it
   converge.
4. Day-1 retention: do they ever decide an approval after
   hire?

---

**File status**: D2 — authored 2026-05-11 as the canonical
spec for the hire-flow surface. Updated alongside any PR
that touches the flow.
