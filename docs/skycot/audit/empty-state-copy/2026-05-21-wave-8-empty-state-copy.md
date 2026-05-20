# Wave-8 Audit — Empty-State Copy & Tone

**Date:** 2026-05-21
**Agent:** Wave-8 audit agent #1
**Scope:** Every empty-state surface across the in-product `(app)` routes
**Mode:** Read-only — no code touched
**Brand-voice frame:** "AI teammates, not tools — colleagues with names + personality"

Scoring rubric (each 0–2, total 0–10):
1. **Teammate language** — uses "teammate"/named worker, not "workers"/"tools"/"bots" (0–2)
2. **Specificity** — names *what* is missing, not "items"/"data" (0–2)
3. **Action-oriented** — gives an explicit next step or CTA (0–2)
4. **Friendly tone** — warm, conversational, not robotic (0–2)
5. **Concise + on-brand wit** — one sentence + one CTA, with personality where appropriate (0–2)

---

## Per-surface findings

### 1. `/` (Approvals dashboard) — idle state
**File:** `~/code/argo/web/app/(app)/page.tsx:280-300`

> "All clear" — "Your teammates have nothing waiting on you. Get back to work — they've got the rest."

- Teammate language: 2/2 ("teammates")
- Specificity: 1/2 (generic "All clear" — doesn't name what's clear)
- Action: 1/2 (implicit "go work" but no CTA button)
- Friendly: 2/2 (genuine warmth — "get back to work")
- Concise + wit: 2/2 (the "they've got the rest" line is the best in the whole product)

**Score: 8/10** — Strong. The pulled-from-the-team-leader voice is exactly the brand. The only ding is that "All clear" is generic for a brand built on naming things.

---

### 2. `/` (Approvals dashboard) — filtered/searched empty
**File:** `~/code/argo/web/app/(app)/page.tsx:280-300`

> "No matching approvals" — `Nothing matches "{query}". Try different keywords or clear the filter.`

- Teammate language: 0/2
- Specificity: 1/2 (echoes query at least)
- Action: 2/2 (Clear all button)
- Friendly: 1/2
- Concise + wit: 1/2

**Score: 5/10** — Mechanical. Could mention the teammate context.

**Suggested:** *"No approvals match "{query}". Your teammates may have wrapped them already — clear the filter to look across all of them."*

---

### 3. `/team` first-hire empty
**File:** `~/code/argo/web/components/skycot/TeamEmptyState.tsx`

> "Hire your first AI teammate." / "Pick a starting point — you can adjust everything in the next minute."
> Feature bullets including: *"Pause or adjust her anytime — she's your teammate, not your tool."*

- Teammate language: 2/2
- Specificity: 2/2 (5 example role tiles!)
- Action: 2/2 (5 tile CTAs + "describe your own")
- Friendly: 2/2 (warm, low-pressure)
- Concise + wit: 2/2

**Score: 10/10** — **Gold standard.** Best empty-state in the product. The "she's your teammate, not your tool" bullet is the brand articulated in one sentence.

---

### 4. `/team` no-search-matches
**File:** `~/code/argo/web/app/(app)/team/page.tsx`

> "No matching teammates" — `Nothing matches "{filters.q}". Try a different search or clear the filter.`

- Teammate language: 2/2
- Specificity: 1/2 (echoes query)
- Action: 2/2 (Clear all)
- Friendly: 1/2
- Concise + wit: 1/2

**Score: 7/10** — Acceptable. Could be warmer.

---

### 5. `/team` no-status-matches
**File:** `~/code/argo/web/app/(app)/team/page.tsx`

> "No teammates in this state right now."

- Teammate language: 2/2
- Specificity: 1/2
- Action: 1/2 (Clear all only)
- Friendly: 1/2
- Concise + wit: 1/2

**Score: 6/10** — Functional but flat.

**Suggested:** *"None of your teammates are paused right now — everyone's working."* (vary per status)

---

### 6. `/workers` first-load empty
**File:** `~/code/argo/web/app/(app)/workers/page.tsx`

> "No teammates yet" / "Hire your first AI worker to start shipping." / CTA: "Hire a worker"

- Teammate language: 1/2 (headline says "teammates" but body says "worker")
- Specificity: 1/2
- Action: 2/2 (CTA)
- Friendly: 1/2
- Concise + wit: 1/2

**Score: 6/10** — Mixed metaphor: headline uses "teammates" but body and CTA say "worker". Inconsistent vs. `/team` which is the brand exemplar. This page should probably redirect to `/team` or share the `TeamEmptyState` component.

**Suggested:** *"No teammates yet. Hire your first one — she'll be in your Slack in four minutes."* CTA: *"Hire your first teammate"*

---

### 7. `/workflows` filtered empty
**File:** `~/code/argo/web/app/(app)/workflows/page.tsx`

> `No workflows match "{query}".` / "Try a different query or browse the 50+ templates."

- Teammate language: 0/2
- Specificity: 2/2 (echoes query + names the "50+ templates" alternative)
- Action: 2/2 (browse-templates button)
- Friendly: 1/2
- Concise + wit: 1/2

**Score: 6/10** — Solid for a filter-empty state. Workflows is a more abstract surface so teammate language is harder to apply naturally.

---

### 8. `/audit` pristine
**File:** `~/code/argo/web/app/(app)/audit/page.tsx`

> "Quiet day" / "Worker actions and approvals will appear here as they happen. Plenty of room for activity."

- Teammate language: 0/2 ("Worker actions" is the tool framing)
- Specificity: 1/2
- Action: 0/2 (no CTA — by design, it's pristine)
- Friendly: 2/2 ("Quiet day" + "plenty of room" is warm)
- Concise + wit: 2/2 ("Quiet day" is a great two-word headline)

**Score: 5/10** — "Quiet day" is brand-perfect; the body line drops back to "worker actions". Mix.

**Suggested:** *"Quiet day. Your teammates' actions and approvals show up here as they happen — plenty of room."*

---

### 9. `/audit` filtered empty
**File:** `~/code/argo/web/app/(app)/audit/page.tsx`

> "No actions match" / dynamic filter description / CTA: "Clear filters"

- Teammate language: 0/2
- Specificity: 2/2 (filtered detail is dynamic)
- Action: 2/2 (Clear filters)
- Friendly: 0/2 (purely mechanical)
- Concise + wit: 1/2

**Score: 5/10** — Tight but tone-flat. Audit is a restrained surface (per locked design) so this is mostly fine — just add a tiny human edge.

---

### 10. `/cohorts` first-load empty
**File:** `~/code/argo/web/app/(app)/cohorts/page.tsx`

> "No cohorts" / "Group similar tasks into a cohort to share context across workers." / CTA: "Create cohort"

- Teammate language: 0/2 ("workers", not "teammates")
- Specificity: 1/2
- Action: 2/2 (CTA)
- Friendly: 1/2
- Concise + wit: 1/2

**Score: 5/10** — Reads like docs. "Share context across workers" is the tool framing.

**Suggested:** *"No cohorts yet. Group teammates who do similar work so they can share what they've learned."* CTA: *"Create your first cohort"*

---

### 11. `/skills` filtered empty
**File:** `~/code/argo/web/app/(app)/skills/page.tsx`

> `No skills match "{query}". Try a different search or clear the filter.`

- Teammate language: 0/2
- Specificity: 1/2
- Action: 1/2 (no button, just "clear the filter" text)
- Friendly: 0/2
- Concise + wit: 0/2

**Score: 2/10** — **Worst empty-state in the product.** Plain `<div>` with one line, no CTA button, no warmth. The Skills page hero is brand-loud-marketplace but the empty falls right off the brand cliff. This is the page Mark called out as the brand "marketplace" surface; the empty doesn't match.

**Suggested:** *"No skills match "{query}". Try a broader term, or pop open the catalog to see what your teammates can learn next."* CTA: *"Clear search"*

---

### 12. `/workers/[id]/audit` (= /activity) pristine
**File:** `~/code/argo/web/app/(app)/workers/[workerId]/activity/page.tsx`

> "No activity yet." / `{workerName} hasn't taken any actions yet. Once she's working, you'll see her activity here.`

- Teammate language: 2/2 (uses worker name, uses "she")
- Specificity: 2/2 (named worker, named action)
- Action: 0/2 (no CTA — pristine)
- Friendly: 2/2
- Concise + wit: 2/2

**Score: 8/10** — Strong. The pronoun + name + "once she's working" cadence is the brand voice.

---

### 13. `/workers/[id]/activity` filtered empty
**File:** `~/code/argo/web/app/(app)/workers/[workerId]/activity/page.tsx`

> "No matching activity" / "Try widening the time range or removing filters." / Clear-filters button

- Teammate language: 0/2
- Specificity: 1/2
- Action: 2/2
- Friendly: 0/2
- Concise + wit: 0/2

**Score: 3/10** — Lost the worker name from the sibling pristine state. Same surface, half the brand voice.

**Suggested:** *"No activity from {workerName} in this window."* / Try widening… (keep button)

---

### 14. `/workers/[id]/memory` pristine
**File:** `~/code/argo/web/app/(app)/workers/[workerId]/memory/page.tsx`

> "Memory is quiet" / `When {workerName} remembers facts or decisions, they appear here.`

- Teammate language: 2/2 (named)
- Specificity: 2/2
- Action: 0/2 (intentional)
- Friendly: 2/2 ("Memory is quiet" — lovely cadence)
- Concise + wit: 2/2

**Score: 8/10** — Strong. Pairs well with the Memory page's restrained register.

---

### 15. `/workers/[id]/memory` facts-empty card
**File:** `~/code/argo/web/components/skycot/worker/MemoryFactsCard.tsx`

> `{workerName} is still learning about your business.` / "Add what she should know above, or let her pick it up as she works."

- Teammate language: 2/2 (named + "she")
- Specificity: 2/2
- Action: 2/2 (two paths: explicit add or implicit learning)
- Friendly: 2/2
- Concise + wit: 2/2

**Score: 10/10** — **Gold standard #2.** "Still learning about your business" is exactly the framing. Two paths surfaced gracefully.

---

### 16. `/workers/[id]/skills` Active card empty
**File:** `~/code/argo/web/components/skycot/worker/SkillsActiveCard.tsx`

> `{workerName} doesn't have any skills assigned yet.`

- Teammate language: 2/2
- Specificity: 1/2
- Action: 0/2 (no CTA)
- Friendly: 1/2
- Concise + wit: 0/2

**Score: 4/10** — Functional but bare. Active skills empty is the moment to point to the Library section right below it.

**Suggested:** *"{workerName} hasn't picked up any active skills yet. Try one from the library below to teach her."*

---

### 17. `/workers/[id]/skills` Library card empty
**File:** `~/code/argo/web/components/skycot/worker/SkillsLibraryCard.tsx`

> "No skills available beyond the ones in use. Add new patterns from the Learned section below."

- Teammate language: 0/2
- Specificity: 1/2 (refers to "Learned section")
- Action: 1/2 (text directive only)
- Friendly: 0/2
- Concise + wit: 0/2

**Score: 2/10** — Reads like an internal jargon dump ("patterns", "in use", "Learned section"). Doesn't fit the marketplace tone.

**Suggested:** *"Nothing in the library yet. Promising patterns from {workerName}'s recent work will appear here for you to try."*

---

### 18. `/workers/[id]/skills` Learned card empty
**File:** `~/code/argo/web/components/skycot/worker/SkillsLearnedCard.tsx`

> `Nothing new yet. {workerName} will suggest patterns here once she's used them successfully a few times.`

- Teammate language: 2/2
- Specificity: 2/2
- Action: 0/2 (intentional — it's an emerging state)
- Friendly: 2/2
- Concise + wit: 2/2

**Score: 8/10** — Strong. "Used them successfully a few times" is the right grounded register.

---

### 19. `/workers/[id]/evals` empty
**File:** `~/code/argo/web/app/(app)/workers/[workerId]/evals/page.tsx`

> "No evals run" / `Run an eval set to grade {workerName}'s work against expected outcomes.` / CTA: "Run an eval"

- Teammate language: 2/2 (named worker)
- Specificity: 2/2
- Action: 2/2 (CTA)
- Friendly: 1/2 ("grade her work" — slightly cold)
- Concise + wit: 1/2

**Score: 8/10** — Strong, just slightly clinical with "grade" and "expected outcomes".

---

### 20. `/workers/[id]/skills/[skillId]/canvas` skill not learned
**File:** `~/code/argo/web/app/(app)/workers/[workerId]/skills/[skillId]/canvas/page.tsx`

> "Worker hasn't learned this skill yet." / `Skill {skillId} has no IR versions on file.` / two CTAs (Teach / Ghostwriter)

- Teammate language: 0/2 ("Worker" — not even the name)
- Specificity: 2/2
- Action: 2/2 (two CTAs)
- Friendly: 0/2 ("IR versions on file" is engineer-speak)
- Concise + wit: 0/2

**Score: 4/10** — The "IR versions on file" line is the worst single phrase across the whole audit — pure backend internal terminology leaking to the UI.

**Suggested:** *"{workerName} hasn't learned this skill yet — there's no playbook on file. Teach her, or have the ghostwriter draft one."*

---

### 21. `/workers/[id]/overview` Recent Activity card empty
**File:** `~/code/argo/web/components/skycot/worker/RecentActivityCard.tsx`

> "No activity yet — she'll show up here once she starts working."

- Teammate language: 2/2 (pronoun)
- Specificity: 1/2
- Action: 0/2 (intentional)
- Friendly: 2/2
- Concise + wit: 2/2

**Score: 7/10** — Tight and warm.

---

### 22. `/workers/[id]/overview` Active Conversations card empty
**File:** `~/code/argo/web/components/skycot/worker/ActiveConversationsCard.tsx`

> "No open conversations right now."

- Teammate language: 0/2
- Specificity: 1/2
- Action: 0/2
- Friendly: 1/2
- Concise + wit: 1/2

**Score: 3/10** — Functional, but no name + no warmth.

**Suggested:** *"{workerName} has no open conversations right now."*

---

### 23. `/workers/[id]/overview` Recent Decisions card empty
**File:** `~/code/argo/web/components/skycot/worker/RecentDecisionsCard.tsx`

> "No material decisions in the last 14 days."

- Teammate language: 0/2
- Specificity: 2/2 (named window)
- Action: 0/2
- Friendly: 0/2 ("material decisions" — corporate-speak)
- Concise + wit: 0/2

**Score: 2/10** — Reads like a board-pack footnote.

**Suggested:** *"{workerName} hasn't made any big calls in the last two weeks — quiet stretch."*

---

### 24. `/workers/[id]/overview` Duties card empty
**File:** `~/code/argo/web/components/skycot/worker/DutiesCard.tsx`

> "No responsibilities yet."

- Teammate language: 0/2
- Specificity: 1/2
- Action: 0/2 (no CTA inline — though there is an Add button in the card header)
- Friendly: 0/2
- Concise + wit: 0/2

**Score: 1/10** — **Worst-scoring single line.** Three words. No name, no warmth, no CTA inline.

**Suggested:** *"{workerName} doesn't have any responsibilities yet. Add the first one to put her to work."*

---

### 25. Notifications popover empty
**File:** `~/code/argo/web/components/skycot/NotificationsPopover.tsx`

> "Nothing new." / "You'll see system notifications here — budget alerts, OAuth renewals, drift warnings."

- Teammate language: 0/2 (system notifications, no teammate framing — arguably correct since this is system-level)
- Specificity: 2/2 (names the three types)
- Action: 0/2
- Friendly: 1/2
- Concise + wit: 1/2

**Score: 4/10** — Acceptable. This is a system surface, not a teammate one, so partial credit on teammate-language is fair.

---

### 26. `/retro` empty (pristine)
**File:** `~/code/argo/web/app/(app)/retro/page.tsx`

> "No retros yet" / "Run a retro from a worker profile after a long-running duty wraps."

- Teammate language: 0/2 ("worker profile", "duty")
- Specificity: 2/2
- Action: 1/2 (text directive)
- Friendly: 0/2
- Concise + wit: 0/2

**Score: 3/10** — Mechanical.

**Suggested:** *"No retros yet. After a teammate wraps a long stretch of work, pop open her profile and run one."*

---

### 27. `/settings/api-keys` empty
**File:** `~/code/argo/web/app/(app)/settings/api-keys/page.tsx`

> "No API keys yet." / "Mint one to wire SKYCOT into your CI pipeline, data warehouse sync, or any integration that needs to call the API without a human session." / CTA: "Create your first key"

- Teammate language: 0/2 (system surface — fair)
- Specificity: 2/2 (three named use cases)
- Action: 2/2 (CTA)
- Friendly: 1/2
- Concise + wit: 1/2

**Score: 6/10** — Solid for a system surface.

---

## Summary statistics

- **Total empty states audited:** 27
- **Mean score:** 5.4 / 10
- **Median score:** 5 / 10
- **Pass (≥7):** 9 surfaces (33%)
- **Below brand bar (<7):** 18 surfaces (67%)
- **Generic-SaaS feel (≤4):** 11 surfaces (41%)
- **Gold standard (10/10):** 2 surfaces — Team empty state + MemoryFactsCard

## Top 5 needing rewrites (lowest scores)

| Rank | Surface | Score | Why |
|---|---|---|---|
| 1 | Duties card empty (`worker/DutiesCard.tsx`) | 1 | Three words: "No responsibilities yet." No name, no CTA, no warmth |
| 2 | `/skills` filtered empty | 2 | Plain `<div>`, no CTA button, mismatches the brand-loud marketplace hero above it |
| 3 | Skills Library card empty (`worker/SkillsLibraryCard.tsx`) | 2 | Internal jargon dump ("patterns", "in use", "Learned section") |
| 4 | Recent Decisions card empty | 2 | "Material decisions" — board-pack language, no teammate framing |
| 5 | `/workers/[id]/activity` filtered empty | 3 | Loses the worker name that the sibling pristine state uses well |

Honorable mentions: `/retro` empty (3), Active Conversations card (3), Skill canvas not-learned (4 — leaks "IR versions on file").

## Best existing copy worth keeping as template

**Winner: `TeamEmptyState` (`/team`)** — the first-hire empty state.

Why it works as the template:
- Locks the headline + sub-line copy in code comments (anti-drift)
- 5 example role tiles named after real personas (specificity)
- Two action paths: pick a tile OR describe your own (action-oriented + low pressure)
- Closed-by-default disclosure with 5 feature bullets including the brand-perfect line *"she's your teammate, not your tool"*
- Used uniformly with `data-testid="team-empty-state"` for tests

**Runner-up:** `MemoryFactsCard` empty — *"{workerName} is still learning about your business. Add what she should know above, or let her pick it up as she works."* — perfect "two paths" framing with named worker + pronoun.

## Patterns that cause low scores

1. **"Worker" instead of "teammate" in body copy** — `/workers`, `/cohorts`, Active Conversations
2. **Backend jargon leaks** — "IR versions on file", "material decisions", "patterns in use"
3. **Three-word stubs** — "No responsibilities yet.", "Nothing new.", "No open conversations right now."
4. **Filter-empty states drop the worker name** that the sibling pristine state uses well
5. **`/workers` page doesn't reuse the `TeamEmptyState` component** — duplicate empty for the same conceptual surface, lower quality version

## Patterns that produce high scores

1. **Use the worker's name + pronoun together** (Memory, Recent Activity, Skills Learned)
2. **"Two paths" framing** — explicit action plus implicit learning (MemoryFactsCard, TeamEmptyState)
3. **Two-word warm headlines** — "Quiet day", "Memory is quiet", "All clear"
4. **Specific time windows** — "in the last 14 days", "next minute", "four minutes"
5. **Brand articulation line in the empty** — "she's your teammate, not your tool"

## Architectural recommendation

Three of the worst surfaces (`/workers`, `/cohorts`, `/skills` filtered) re-implement bespoke empty states when a shared `EmptyState` component already exists. The `/workers` empty in particular should either redirect to `/team` or import the `TeamEmptyState` component — it's the same user moment, lower quality copy.

The `EmptyState` ui primitive (`web/components/ui/EmptyState.tsx`) already accepts `headline`/`body`/`cta`/`illustration` and is well-tested. Migrating the 11 below-bar surfaces to it would force consistency *and* close the drift gap.
