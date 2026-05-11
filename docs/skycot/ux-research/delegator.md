# Persona — the Delegator

> The person who "hires" AI teammates and lives with the
> consequences. Primary daily user. Primary SMB buyer. The reason
> this product exists.

## Who they are

Three archetypes — pick whichever helps you design for the moment:

1. **The marketplace seller, "Sam"** — runs 3 storefronts across
   Facebook Marketplace, Gumtree, eBay. Photographs, lists,
   negotiates, ships. 60 listings active at any moment. Used to
   spend evenings replying to "is this still available?" Now
   wants an AI teammate that does it for him.
2. **The recruiter, "Priya"** — solo recruiter, 5 active searches,
   gets 30 inbound applications per day, every one demands a
   reply within 24h or the candidate goes cold. Has no admin
   support. Wants an AI teammate that screens, schedules, and
   keeps candidates warm.
3. **The dept head, "Marcus"** — runs a 12-person customer-support
   team at a 200-person SaaS. Drowning in Tier-1 tickets. Can't
   get headcount approved. Wants an AI teammate that handles the
   obvious questions and escalates the rest.

## What they have in common

- **Time-starved, not money-starved (within reason).** They will
  pay $50–$500/mo for a tool that gives them their evenings back.
  They will not pay $50/mo for a tool that takes them 4 hours to
  set up.
- **Burnt by previous "AI" promises.** They have seen ChatGPT
  demos. They have tried Zapier-bolted-to-GPT setups. None of it
  ran their actual business. They are skeptical and they are
  right to be.
- **Comfortable with SaaS, allergic to code.** They use Notion,
  Slack, Stripe, QuickBooks. They have never edited a TOML file
  and won't start now. The word "vertical" means *industry*, not
  *configuration template*.
- **Identity-attached to the work.** Every email their AI sends
  goes out under *their* name and *their* reputation. Trust is
  not a feature; it's the entire product.
- **Constantly interrupted.** They check the product between
  customer calls, in line at the coffee shop, in the 90 seconds
  between meetings. The product must work in 90-second slices.

## What they need from SKYCOT

In priority order:

1. **Confidence the worker won't embarrass them.** They need to
   see exactly what the worker is about to do BEFORE it does it,
   for at least the first week. Dry-run mode is not optional.
2. **A clear mental model of "what does my AI teammate know about
   my business?"** Memory must be browsable. "She knows your
   inventory CSV, your pricing rules, and that you don't ship to
   PO boxes" beats "vector store with 4,213 embeddings."
3. **Visible accountability.** When the AI gets it wrong, they
   need to see what it saw, what it decided, and why. Not for
   blame — for fixing. "Black box that sometimes works" is
   disqualifying.
4. **One-click course correction.** When the worker is doing
   something wrong, they fix it by saying "no, don't do that"
   in plain English, not by editing a YAML rule file.
5. **Outcomes, not metrics.** They do not care about token cost
   per turn. They care: how many listings did the worker write
   this week, how many got replies, how much did I make. Frame
   everything in their language.

## What they do NOT want

- A dashboard with 47 metrics.
- A "prompt engineering" interface.
- To learn what an "agent" is, technically.
- To understand the difference between a vertical and a worker.
- To be the QA team for the AI.
- To explain to their accountant what a "token" is.

## Anti-patterns to watch for in design

- **Configuration framed as flexibility.** Every config option you
  expose is a decision you've punted to the user. Default
  aggressively. Hide everything that isn't load-bearing for the
  first 30 days.
- **Empty states that read like "your dashboard is empty, get
  started by creating a workflow."** They don't know what a
  workflow is. The empty state should say: "Hire your first
  teammate. Tell me what you need help with." Click → Genesis
  conversation.
- **Treating the CLI as the source of truth and the web app as a
  view layer.** It's the opposite. The web app is where they
  live. The CLI is the power-user escape hatch.
- **Notification fatigue.** Every push notification not tied to a
  decision they need to make is a step toward uninstall.
- **Confusing "the worker" with "a session" or "a job".** Sam
  has one teammate named *Mariah* who does his listings. He does
  not run sessions. He does not queue jobs. Internal vocabulary
  must not leak.

## Day in the life — Sam, marketplace seller

| Time | Context | What he does | What he needs from SKYCOT |
|---|---|---|---|
| 06:30 | Coffee, phone in hand | Opens SKYCOT app. Sees Mariah's overnight summary: "3 listings drafted, 2 sent (with your approval Tue), 1 waiting on your OK because the price feels high vs comps." | One screen, scannable in 30s, no tabs to switch |
| 07:00 | Driving | Slack DM from a buyer comes in via Mariah: "Is this still available? Can you do $80?" → push notification "Mariah wants to approve a counter-offer at $90" | One-tap approve from lock screen. Audio readout option for hands-free |
| 11:30 | Between shipments | Three listings auto-drafted overnight. He reviews each in 10s by swiping. Two approve, one he edits the title. | Inline edit, no modal, no "save draft" button |
| 14:00 | Customer support eats his afternoon | Mariah handles 6 inbound messages autonomously per the auto-approval policy. He sees them in the activity feed but doesn't act. | Quiet activity feed. NO push for things she's allowed to do. |
| 18:00 | Closing the day | Quick glance: "Mariah did 14 things today, generated $340 revenue, asked you 4 times, you said yes 3 times. Tomorrow: 2 follow-ups scheduled." | Outcome framing in his language: revenue, replies, follow-ups. |
| 22:00 | Phone in bed | Late buyer message arrives. Quiet hours respected → no notification. Will be the first thing he sees at 06:30. | Quiet-hours channels MERGED in #1264 — already wired. UI just needs to surface it. |

## Critical moments

The three points where Sam will quit and never return:

1. **First-run friction > 10 minutes.** If hiring Mariah requires
   reading docs, picking a "vertical", or configuring anything,
   he's gone.
2. **Mariah's first action embarrasses him.** If she sends a
   listing with a typo, a wrong price, or a tone that's not his,
   on Day 1 with no approval gate, he uninstalls.
3. **He can't figure out what she did when something goes
   wrong.** If a buyer complains about a message and Sam can't
   find the message in the app within 30 seconds, he loses
   trust permanently.

The product's job is to make these three moments impossible.

## What we don't know yet

`(assumed — needs validation)` flags on the claims above. The
following need real-user conversations to confirm or correct:

- Pricing tolerance ($50–$500/mo range).
- Notification cadence (is the morning summary too much? too
  little?).
- Whether "she" / "he" / "they" naming actually helps or feels
  weird in practice.
- Whether the 30-day "approval-gate everything" default feels
  reassuring or frustrating.

Plan to interview 3 Sams, 3 Priyas, 3 Marcuses before the
implementation phase locks visual direction.
