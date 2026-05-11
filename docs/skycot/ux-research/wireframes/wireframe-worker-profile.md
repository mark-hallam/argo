# Wireframe — Worker Profile

The deep view of one AI teammate. Persona + role + duties + KPIs
+ memory + skills + recent activity + entry points to retrain
and delegate-more. The most data-dense surface in v1.

## Purpose

Show everything the Delegator can know about, and do to, one
teammate. Anchor the "AI as employee" mental model: a worker
has identity, a job description, accumulated knowledge, learned
skills, and a paper trail. Make those four things browsable,
verifiable, and editable.

**Serves:**
- Persona: [delegator.md](../delegator.md) (primary),
  [operator.md](../operator.md) (light — operators view this for
  team members they're administering),
  [auditor.md](../auditor.md) (light — links into the audit
  timeline for this worker)
- Journey:
  [journey-delegate-ongoing-work.md](../journey-delegate-ongoing-work.md)
  (entry point for "+ Add responsibility"),
  [journey-retrain-a-worker.md](../journey-retrain-a-worker.md)
  (entry point for "Adjust how she works"),
  [journey-audit-replay.md](../journey-audit-replay.md) (linked
  from Activity tab).

**Implementation primitives:**
- [Card](../../../../web/components/ui/Card.tsx),
  [Avatar](../../../../web/components/ui/Avatar.tsx),
  [Button](../../../../web/components/ui/Button.tsx),
  [StatusPill](../../../../web/components/ui/StatusPill.tsx)
- NEW: `Tabs` (from shadcn), `KpiSparkline`, `MemoryBrowser`,
  `SkillRow`, `DutyRow`, `ActivityRow`, `RetrainDialog`,
  `AddResponsibilityDialog`.

## Information architecture

- URL: `/workers/<workerId>` — defaults to Overview tab.
- Tabs as sub-routes: `/workers/<workerId>/<tab>` for
  `overview` / `activity` / `memory` / `skills` / `settings`.
- Breadcrumb: `Team › Mariah`. Team links back to `/team`.
- Linked from:
  - Team page card click.
  - Approvals inbox: worker name in any approval card →
    `/workers/<workerId>`.
  - Audit timeline: any row's worker link.
- Deep-links back to:
  - Approvals filtered to this worker:
    `/?worker=<workerId>`.
  - Audit filtered to this worker:
    `/audit?worker=<workerId>`.

## Layout (default state, Overview tab, desktop)

```
+----------------------------------------------------------------+
| (sidebar same as always)                                       |
|                              |                                 |
|                              |  Team › Mariah                  |
|                              |                                 |
|                              |  ╔═══════════════════════════╗ |
|                              |  ║ (M)  Mariah               ║ |
|                              |  ║      Marketplace assistant║ |
|                              |  ║      ● Working   $340 today║|
|                              |  ║                            ║ |
|                              |  ║                   [⋮]      ║ |
|                              |  ╚═══════════════════════════╝ |
|                              |  (1) HERO                       |
|                              |                                 |
|                              |  [Overview][Activity][Memory]   |
|                              |  [Skills][Settings]             |
|                              |  (2) TABS                       |
|                              |                                 |
|                              |  +-- WHAT SHE DOES -----------+ |
|                              |  | Charter (3 sentences)       |
|                              |  |                              |
|                              |  | Mariah helps Sam manage     |
|                              |  | his marketplace stores...   |
|                              |  | [✎ Adjust]            (3)   |
|                              |  +-----------------------------+ |
|                              |                                 |
|                              |  +-- DUTIES (4) ---------------+ |
|                              |  | □ Read inbound msgs 30min   |
|                              |  | □ Draft "still available?"  |
|                              |  | □ Flag counters < floor      |
|                              |  | □ Evening price-check       |
|                              |  | [+ Add responsibility]      |
|                              |  +-----------------------------+ |
|                              |                                 |
|                              |  +-- KPIs (5) -----------------+ |
|                              |  | Replies sent        128  ⬈  |
|                              |  | Approval rate       94%  ⬈  |
|                              |  | Revenue this week  $1.2k ⬈  |
|                              |  | Avg response time  4m   ⬊  |
|                              |  +-----------------------------+ |
|                              |                                 |
|                              |  +-- RECENT ACTIVITY ---------+ |
|                              |  | [link list of last 5      ] |
|                              |  | [actions, each click→audit ] |
|                              |  | [See all in Activity →    ] |
|                              |  +-----------------------------+ |
+----------------------------------------------------------------+
```

## Annotations — Hero + Tabs

**(1)** Hero card. Distinguishing element: thicker border or
slight gradient (uses `--primary` at 5-10% alpha as
background tint). Contains:

- Avatar lg (with status ring per current worker state).
- Name + role tagline.
- Status pill (Working / Waiting / Idle / Blocked / Paused).
- Today KPI summary: "{revenue}" or other vertical-specific
  primary KPI inline next to status.
- Overflow menu `⋮` (top-right of hero): Pause, Disable
  duties, Archive teammate, Export her data (GDPR-Article-15
  scoped to this worker), View as Operator.

**(2)** Tab navigation. 5 tabs:
- `Overview` (default)
- `Activity` (recent actions, links to audit)
- `Memory` (what she knows)
- `Skills` (what she can do)
- `Settings` (channels, policies, advanced config)

Selected tab styling: accent border-bottom + accent
foreground. Unselected: muted-foreground.

Uses shadcn `Tabs` primitive.

## Overview tab — annotations

**(3)** Charter card. The free-text 2-3 sentence statement
of what the worker does. Click "Adjust" to open
RetrainDialog (see "Adjust path" below).

**(4)** Duties list. Each duty:
- Toggle (□/☑ checkbox) — enabled/disabled.
- Plain-English description.
- Schedule (cron / event trigger) shown as a small chip
  on the right ("every 30m", "morning", "on inbound").
- Click duty row → opens duty editor inline.
- `[+ Add responsibility]` button at the bottom — opens
  AddResponsibilityDialog (see
  [journey-delegate-ongoing-work.md](../journey-delegate-ongoing-work.md)).

**(5)** KPI block. 4 KPIs displayed inline:
- Per-vertical default set. Marketplace shows: replies
  sent, approval rate, revenue this week, avg response
  time.
- Each KPI has a sparkline showing last 7-day trend.
- Up-arrow / down-arrow with semantic color (success for
  improving, warning for regressing).
- Click a KPI → opens detailed view (out of v1; routes to
  a deeper `/workers/<id>/kpis/<kpiName>` route in v2).

**Recent activity (bottom of Overview)**: condensed list of
last 5 actions. Each row is a single line: timestamp +
verb + status. Click any row → audit drill-in. "See all in
Activity →" link routes to the Activity tab.

---

## Activity tab

```
+----------------------------------------------------------------+
|                              |  Team › Mariah › Activity       |
|                              |                                 |
|                              |  (hero, condensed)              |
|                              |  [Overview][Activity*][Memory]  |
|                              |                                 |
|                              |  [ 🔍 Filter ] [ Today v ]      |
|                              |                                 |
|                              |  +-- TIMELINE ----------------+ |
|                              |  | • 14:32 sent reply to Aroha │ |
|                              |  |   buyer #4821 · approved   │ |
|                              |  |   by you · 3m ago          │ |
|                              |  | • 13:50 drafted listing for │ |
|                              |  |   Bauer Stick · pending OK │ |
|                              |  | • 09:15 morning inbox      │ |
|                              |  |   summary · autonomous     │ |
|                              |  | ... 20 more rows ...        │ |
|                              |  | [ Load more ]                │ |
|                              |  +-----------------------------+ |
+----------------------------------------------------------------+
```

This tab is a worker-filtered slice of the
[audit timeline](wireframe-audit-timeline.md). Filters
default to this worker + today. Reuses the ActivityRow
component. Click any row routes to the audit drill-in for
that action.

---

## Memory tab

```
+----------------------------------------------------------------+
|                              |  Team › Mariah › Memory         |
|                              |                                 |
|                              |  (hero condensed)               |
|                              |  [Overview][Activity][Memory*]  |
|                              |                                 |
|                              |  Mariah knows:                  |
|                              |                                 |
|                              |  +-- FACTS ------------------+  |
|                              |  | • Your storefront is in   |  |
|                              |  |   Auckland.                |  |
|                              |  | • Price floor: $50 unless |  |
|                              |  |   you OK lower.            |  |
|                              |  | • No PO box shipping.     |  |
|                              |  | • Inventory CSV (links to |  |
|                              |  |   `/inventory.csv`).      |  |
|                              |  | [+ Add a fact]            |  |
|                              |  +-----------------------------+ |
|                              |                                 |
|                              |  +-- ACTIVE CONVERSATIONS ---+  |
|                              |  | • Aroha — Bauer Vapor X5  |  |
|                              |  |   (2 msgs, last 3m ago)   |  |
|                              |  | • Diego — CCM helmet      |  |
|                              |  |   (1 msg, last 1h ago)    |  |
|                              |  +-----------------------------+ |
|                              |                                 |
|                              |  +-- RECENT DECISIONS -------+  |
|                              |  | • Approved counter @ $85   |  |
|                              |  |   to Aroha (May 10)        |  |
|                              |  | • Refused PO box shipping |  |
|                              |  |   to Lina (May 9)         |  |
|                              |  +-----------------------------+ |
|                              |                                 |
|                              |  Tone: warm, casual, signs off  |
|                              |  with "Cheers!"  [✎ Adjust]     |
+----------------------------------------------------------------+
```

The Memory tab is **plain-English facts**, never
embeddings or vectors. Three sub-sections:

1. **Facts** — explicit memory entries the operator
   added or the worker learned. Each row is editable
   (click to expand → edit / delete). `[+ Add a fact]`
   button opens a free-text dialog.
2. **Active conversations** — open threads with end-users
   the worker is mid-conversation in. Click → see the
   thread.
3. **Recent decisions** — material decisions the worker
   has made recently. Click → audit drill-in.

Plus a Voice profile summary at the bottom with `[✎
Adjust]` button → opens the voice editor (subset of
RetrainDialog).

For the operator persona (multi-tenant deployments),
adding the "Memory namespace" row at the top shows the
namespace ID + isolation guarantee badge ("tenant-scoped,
not cross-readable").

---

## Skills tab

```
+----------------------------------------------------------------+
|                              |  Team › Mariah › Skills         |
|                              |                                 |
|                              |  (hero condensed)               |
|                              |  [Overview][Activity][Memory]   |
|                              |  [Skills*][Settings]            |
|                              |                                 |
|                              |  Mariah uses 4 skills:          |
|                              |                                 |
|                              |  +-- ACTIVE -----------------+  |
|                              |  | ⭐ marketplace-reply-quote │  |
|                              |  |   v3 · trust: 92% · used  │  |
|                              |  |   128× last 7d            │  |
|                              |  |   [Disable] [View] [Audit]│  |
|                              |  |                            │  |
|                              |  | ⭐ marketplace-counter-offer│  |
|                              |  |   v2 · trust: 87% · used  │  |
|                              |  |   12× last 7d              │  |
|                              |  +-----------------------------+ |
|                              |                                 |
|                              |  +-- LIBRARY (available) -----+  |
|                              |  | + draft-followup-email     │  |
|                              |  |   v1 · trust: 85% · global│  |
|                              |  |   [Try it]                │  |
|                              |  +-----------------------------+ |
|                              |                                 |
|                              |  +-- LEARNED (pending review)-+  |
|                              |  | new pattern: "decline      │  |
|                              |  | extended-warranty asks"   │  |
|                              |  | 5 successful uses · 0 fails│  |
|                              |  | [Promote] [Discard]       │  |
|                              |  +-----------------------------+ |
+----------------------------------------------------------------+
```

Three sub-sections:

1. **Active** — skills Mariah currently uses. Per row:
   skill name, version, trust score, usage count, action
   buttons (Disable, View source, Audit usage).
2. **Library** — skills available from the workspace or
   marketplace that Mariah doesn't currently use. Per row:
   skill name, version, trust score, source (workspace /
   shared / marketplace), `[Try it]` button.
3. **Learned** — novel patterns the worker has used
   successfully N times that could be promoted to skills.
   Operator can `[Promote]` (publishes to the workspace
   library) or `[Discard]` (the pattern doesn't get
   surfaced again).

Trust score format: percentage. Color: green > 90%,
yellow 70-90%, red < 70%. Trust is derived from the
skill-quality system shipped via mission-skill-quality-*
briefs.

---

## Settings tab

```
+----------------------------------------------------------------+
|                              |  Team › Mariah › Settings       |
|                              |                                 |
|                              |  (hero condensed)               |
|                              |  [Overview]...[Settings*]       |
|                              |                                 |
|                              |  Channels for approvals:        |
|                              |  [✓ Slack] [□ iMessage] [✓ SMS] |
|                              |  Primary: Slack                 |
|                              |                                 |
|                              |  Quiet hours:                   |
|                              |  22:00 – 07:00 NZST  [✎ Edit]   |
|                              |                                 |
|                              |  Budget:                        |
|                              |  $5 / day  $100 / month         |
|                              |  Used: $0.43 today  [✎ Edit]    |
|                              |                                 |
|                              |  Autonomy level:                |
|                              |  ( ) Conservative — ask often   |
|                              |  (•) Balanced — current         |
|                              |  ( ) Aggressive — minimal       |
|                              |                                 |
|                              |  Advanced:                      |
|                              |  [ View role manifest (TOML) ]  |
|                              |  [ Trajectory replay sandbox ]  |
|                              |  [ Worker ID + provenance ]     |
|                              |                                 |
|                              |  Danger zone:                   |
|                              |  [ ⏸ Pause Mariah ]              |
|                              |  [ 🗄 Archive Mariah ]            |
|                              |  [ 🗑 Delete Mariah (operator)]  |
+----------------------------------------------------------------+
```

The Settings tab is the operator-power-user surface. Most
delegators rarely visit. Sections:

- **Channels** — multi-select with one designated as
  primary.
- **Quiet hours** — single range, tz-aware. Inherits from
  workspace by default; per-worker override.
- **Budget** — daily + monthly LLM-cost caps with usage
  bar.
- **Autonomy level** — 3-state radio that adjusts the
  trust-ratchet aggression parameter.
- **Advanced** — escape-hatch entries: view the role
  manifest as raw TOML (read-only for delegators, edit
  for operators), launch trajectory replay, view worker
  ID + full provenance for compliance.
- **Danger zone** — Pause (reversible), Archive (soft-
  delete, recoverable for 30 days), Delete (operator-
  only, permanent). Each requires confirm dialog.

---

## Adjust path — RetrainDialog

Triggered from the Charter card's `[✎ Adjust]` button, or
from the Voice section in Memory tab. Implements
[journey-retrain-a-worker.md](../journey-retrain-a-worker.md).

```
+-----------------------------+
| Adjust how Mariah works     |
|                             |
| What would you change?      |
| +--------------------------+|
| | never agree to ship to PO||
| | boxes                    ||
| +--------------------------+|
|                             |
| [ Continue → ]              |
+-----------------------------+
```

Classified, conflict-checked, regression-tested, then
shown as a plain-English proposal (per journey doc steps
3-5). Final commit lands a new entry in the worker's
"Recent changes" panel at the foot of the Overview tab
with a 14-day undo button.

---

## Add responsibility path — AddResponsibilityDialog

Triggered from the Duties block's `[+ Add responsibility]`
button. Implements
[journey-delegate-ongoing-work.md](../journey-delegate-ongoing-work.md).

```
+-----------------------------+
| Add a responsibility        |
|                             |
| What else do you want       |
| Mariah to handle?           |
| +--------------------------+|
| | summarize end-of-day      ||
| | listings into Slack at 6pm||
| +--------------------------+|
|                             |
| [ Continue → ]              |
+-----------------------------+
```

Same shape as RetrainDialog but classified as "duty
addition" path through Genesis-lite. Shipped with a
cron-or-event picker if the classification suggests one
("at 6pm" → cron `0 18 * * *` proposed; operator
confirms).

---

## States

### Loading state

Hero loads instantly from cached worker data; tabs load
on switch. Each tab's first paint shows skeleton
appropriate to its content (faces row for Memory, table
for Activity, etc).

### Loading state — initial cold load

If the worker data isn't cached, show a centered loading
indicator over the whole content area below breadcrumb +
tabs.

### Worker not found

URL `/workers/<id>` for a nonexistent id:
- Title: `"Worker not found"`.
- Body: `"The worker '${id}' doesn't exist or you don't
  have access."`.
- CTA: `[ Back to team ]` → `/team`.

### Error state per tab

Each tab independently catches errors. The hero stays;
the tab body shows an error card with retry. Other tabs
still navigable.

### Permission-denied state

If user is operator-only and viewing a worker from a
different tenant (cross-tenant scenarios), surface a
permission-denied card with the worker's id + tenant
they belong to (no detail), and a `[ Back to team ]`
escape.

### Paused state

If the worker is paused:
- Hero shows `Paused` status pill with the pause
  timestamp.
- Banner above tabs: `"Mariah is paused. She won't take
  new actions until you resume her. [ Resume ]"`.
- All "schedule new" actions (Add responsibility, etc.)
  disabled.

---

## Mobile layout (< 768px)

- Side rail collapses to hamburger.
- Hero card stacks vertically: avatar centered top,
  name + role below, status + KPI below that, overflow
  menu top-right.
- Tabs become a horizontally-scrolling rail (no overflow
  menu — all 5 tabs fit at 375px because each is ~70px
  wide with abbreviation if needed).
- Each Overview section becomes full-width.
- KPIs stack 2x2 grid instead of 4-in-a-row.
- Memory tab: facts list full-width, sparklines hidden
  on phones.
- Activity tab: timeline takes full width.
- Skills tab: cards stack 1-up.
- Settings tab: works as-is, just narrower.

Touch targets ≥ 44px everywhere. RetrainDialog and
AddResponsibilityDialog become full-screen modals on
mobile.

---

## Interactions + flows

### Tab switching

URL updates. State within each tab is preserved on
return within the session. Cross-session, tabs return to
defaults.

### Adjust (retrain)

Charter card → `[✎ Adjust]` → RetrainDialog → classify →
preview → conflict + regression checks → commit. See
[journey-retrain-a-worker.md](../journey-retrain-a-worker.md).
The change appears in a "Recent changes" panel at the
bottom of the Overview tab with undo for 14 days.

### Add responsibility

Duties block → `[+ Add responsibility]` →
AddResponsibilityDialog → classify → preview → commit.
See [journey-delegate-ongoing-work.md](../journey-delegate-ongoing-work.md).

### Skill promotion

Learned section → `[Promote]` on a learned pattern →
confirm dialog showing 3 sample outputs the skill would
produce → operator picks `[Promote]` again → skill
appears in Active. Audited.

### Pause / archive / delete

Settings → Danger Zone buttons. Each requires confirm
dialog with explicit consequences. Pause is reversible;
archive is soft-delete recoverable for 30 days; delete
is operator-only and permanent.

### Memory edit / delete

Memory → Facts → click a row → expanded edit form →
save / delete. Audited per change.

---

## Empty + error copy (exact strings, locked)

| Section | State | Strings |
|---|---|---|
| Memory | empty facts | Title: `"Mariah is still learning about you."` · Body: `"Add what she should know above, or let her pick it up as she works."` |
| Memory | empty active convos | `"No open conversations right now."` |
| Memory | empty recent decisions | `"No material decisions in the last 14 days."` |
| Skills | empty active | `"Mariah hasn't been assigned any skills yet. Add some from the Library below."` |
| Skills | empty library | `"No skills available beyond the ones Mariah uses. Add new patterns from her Learned section."` |
| Skills | empty learned | `"Nothing new yet. Mariah suggests skills here once she's used a pattern successfully a few times."` |
| Activity | empty | `"No activity yet."` |
| Settings | budget exceeded warning | `"You're approaching your daily budget. Mariah will pause if she hits the limit."` (yellow) / `"Daily budget reached. Mariah is paused until tomorrow or until you raise the limit."` (red) |
| Pause confirm | dialog | Title: `"Pause Mariah?"` · Body: `"She'll stop taking new actions until you resume her."` · CTAs: `[ Cancel ]` `[ Pause ]` |
| Archive confirm | dialog | Title: `"Archive Mariah?"` · Body: `"Her memory and skills are preserved. You can recover her within 30 days."` · CTAs: `[ Cancel ]` `[ Archive ]` |
| Delete confirm (operator) | dialog | Title: `"Delete Mariah permanently?"` · Body: `"This cannot be undone. Audit logs remain but Mariah's memory and learned skills are erased. Type the worker's name to confirm: ___"` · CTAs: `[ Cancel ]` `[ Delete ]` |

---

## Implementation notes for Track E

The largest surface in v1. Track E brief slicing:

- **E4-a: Worker profile — route + hero + tabs shell.**
  Route at `/workers/<id>`, hero card, tab navigation,
  breadcrumb. `useWorker(id)` hook with mock data.
- **E4-b: Worker profile — Overview tab.** Charter card
  + Duties list + KPI block + recent activity.
- **E4-c: Worker profile — Activity tab.** Worker-
  filtered timeline. Reuses the activity-timeline
  primitive.
- **E4-d: Worker profile — Memory tab.** Facts list,
  Active conversations, Recent decisions, Voice
  summary. Memory-edit dialog.
- **E4-e: Worker profile — Skills tab.** Three
  sub-sections, SkillRow component, promote/discard
  flow.
- **E4-f: Worker profile — Settings tab.** Channels,
  quiet hours, budget, autonomy level, danger zone.
- **E4-g: Worker profile — RetrainDialog.** Free-text
  input → classified → previewed → committed. References
  journey-retrain doc.
- **E4-h: Worker profile — AddResponsibilityDialog.**
  Same shape as RetrainDialog but for duty additions.
- **E4-i: Worker profile — KpiSparkline primitive.**
  Reusable sparkline + delta arrow for KPI rows.

9 briefs. Most ≤ 300 LOC. Worker profile is the surface
most likely to need iteration after first delegators
see it, so brief E4-a should ship with feature flags for
each tab to enable progressive rollout.

New primitives:
- `Tabs` (from shadcn).
- `KpiSparkline` (~80 LOC).
- `MemoryBrowser` (~150 LOC).
- `SkillRow` (~120 LOC).
- `DutyRow` (~100 LOC).
- `ActivityRow` (~80 LOC — shared with audit timeline).
- `RetrainDialog` (~250 LOC).
- `AddResponsibilityDialog` (~200 LOC).

---

## Open questions

`(needs validation)`

- Are 5 tabs the right number, or should Skills + Memory
  + Settings collapse into one "Knowledge & Config" tab?
- Should the KPI block be vertical-aware (different
  metrics per vertical) or fixed?
- Should Memory facts be operator-editable or read-only
  (worker-managed)? Default: operator-editable with
  audit trail.
- Should the danger zone require typing the worker's
  name for archive too, or only delete?
- Should the Adjust dialog appear inline (within the
  Charter card) or as a drawer? Drawer feels heavier;
  inline might be too constrained.
- Should the trajectory-replay sandbox button live in
  Settings or in the Activity tab?

Plan to validate by:
1. Heatmap clicks on tabs across first 50 worker-profile
   sessions.
2. Track Adjust + Add-responsibility usage to see if
   the entry points are discoverable.
3. Settings tab visit-rate — if < 10%, the entry should
   be deprioritized in the nav.

---

**File status**: D4 — authored 2026-05-11 as the canonical
spec for the worker-profile surface. Updated alongside any
PR that touches the page.
