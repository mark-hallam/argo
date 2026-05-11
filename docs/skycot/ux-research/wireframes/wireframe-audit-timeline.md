# Wireframe — Audit Timeline

The forensic + compliance surface. Three linked sub-surfaces:
**Activity feed** (chronological list), **Action drill-in**
(everything about one action), **Replay sandbox** (re-run the
action in isolation). Serves operator daily investigation and
auditor quarterly evidence both.

## Purpose

Answer "what happened, why, on whose authority, and would it
happen the same way today?" — fast enough that producing
evidence approaches zero cost. Backbone of the auditor persona
and the primary investigation tool for the operator persona.

**Serves:**
- Persona: [operator.md](../operator.md) (primary — daily
  investigation), [auditor.md](../auditor.md) (primary —
  quarterly evidence), [delegator.md](../delegator.md) (light
  — reached via worker-profile Activity tab)
- Journey: [journey-audit-replay.md](../journey-audit-replay.md)
  (canonical)

**Implementation primitives:**
- [Card](../../../../web/components/ui/Card.tsx),
  [Button](../../../../web/components/ui/Button.tsx),
  [StatusPill](../../../../web/components/ui/StatusPill.tsx),
  [DataCell](../../../../web/components/ui/DataCell.tsx)
- NEW: `ActivityRow` (composite, shared with worker profile),
  `FilterBar`, `EvidenceBundleExporter`, `ReplayDiff`,
  `ProvenanceBlock`.

## Information architecture

- URL: `/audit` → activity feed (the timeline).
- `/audit/action/<actionId>` → drill-in.
- `/audit/action/<actionId>/replay` → replay sandbox.
- Nav placement: 4th item in side rail.
- Breadcrumb: `Audit › <action verb summary>` on drill-in.
- Deep-link filters: `?worker=<id>`, `?tenant=<id>`,
  `?channel=<x>`, `?action_type=<x>`, `?from=<iso>`,
  `?to=<iso>`, `?status=<x>`, `?q=<search>`.
- Linked from:
  - Worker profile Activity tab (worker-scoped).
  - Approval card "Why?" expansion (action-scoped).
  - Operator console (admin landing).
  - Auditor's CLI compliance export tool (URL outputs).

## Layout — Activity feed (default state, desktop)

```
+----------------------------------------------------------------+
| (sidebar same as always)                                       |
|                              |                                 |
|                              |  Audit                          |
|                              |  Forensic + compliance view     |
|                              |                          (1)    |
|                              |                                 |
|                              |  [ Filter v ][ Today v ][ ⤓ Export]|
|                              |  (2)                            |
|                              |                                 |
|                              |  [ 🔍 Search action, worker,    |
|                              |    or action ID...           ]  |
|                              |  (3)                            |
|                              |                                 |
|                              |  +------ TIMELINE (4) ---------+ |
|                              |  | TODAY                       │ |
|                              |  | 14:32 • Mariah sent reply  │ |
|                              |  |   buyer #4821 Slack-approved│ |
|                              |  |   by Sam Harlow · $0.012   │ |
|                              |  |   [act_01H9Z7]      → drill│ |
|                              |  | 13:50 • Mariah drafted     │ |
|                              |  |   listing · pending OK     │ |
|                              |  |   [act_01H9Z6]              │ |
|                              |  | 13:42 • Priya proposed     │ |
|                              |  |   interview slot · awaiting│ |
|                              |  |   approval                  │ |
|                              |  | ... etc ...                │ |
|                              |  |                              │ |
|                              |  | YESTERDAY                    │ |
|                              |  | ... 12 rows ...              │ |
|                              |  |                              │ |
|                              |  | [ Load more (page 1 of 12) ]│ |
|                              |  +-----------------------------+ |
+----------------------------------------------------------------+
```

## Annotations — Activity feed

**(1)** Page title. Strings: `"Audit"`. Subtitle:
`"Forensic + compliance view"`.

**(2)** Action row:
- Filter dropdown. Strings: `"Filter v"`. Opens a popover
  with: Worker · Channel · Action type · Status ·
  Approver · Cost band · Data classification. Multi-
  select per dimension. URL state.
- Time-range dropdown. Strings: `"Today v"`. Options:
  Last hour · Today · Yesterday · Last 7 days · Last 30
  days · This quarter · Last quarter · Custom. URL
  state.
- `[ ⤓ Export ]` button. Opens
  EvidenceBundleExporter dialog (see "Export flow"
  below).

**(3)** Search input. Strings: placeholder `"Search
action, worker, or action ID..."`. Full-text search
across worker.name + verb + action ID + tags. URL
`?q=`. Debounced 200ms (heavier than approvals search
because the corpus is larger).

**(4)** Timeline. Server-paginated 50 rows per page.
Grouped by day with sticky day-headers. Each row is an
ActivityRow:

```
14:32 • Mariah sent reply  buyer #4821 Slack-approved
        by Sam Harlow · $0.012  [act_01H9Z7] →
```

Components of an ActivityRow:
- Timestamp (clickable; copies ISO-8601 to clipboard).
- Worker avatar (xs) + name (link to worker profile).
- Action verb (one-line plain English).
- Outcome status (`approved` / `refused` / `autonomous` /
  `failed` / `pending`).
- Approver (if any — links to user profile).
- Cost (if surfaced — clickable for breakdown).
- Action ID monospace, click to copy.
- Arrow → drill-in.

Row hover state: subtle background highlight; cursor
pointer. Click anywhere on the row → drill-in.

---

## Layout — Action drill-in (default state, desktop)

```
+----------------------------------------------------------------+
|                              |  Audit › Mariah's reply to     |
|                              |  buyer #4821                    |
|                              |                                 |
|                              |  ╔═══════════════════════════╗ |
|                              |  ║ (M) Mariah sent reply     ║ |
|                              |  ║     to buyer #4821 on     ║ |
|                              |  ║     Facebook Marketplace  ║ |
|                              |  ║                            ║ |
|                              |  ║ approved by Sam Harlow    ║ |
|                              |  ║ via Slack, 14:32 NZST     ║ |
|                              |  ║ act_01H9Z7      [Copy ID] ║ |
|                              |  ╚═══════════════════════════╝ |
|                              |  (1) SUMMARY                    |
|                              |                                 |
|                              |  [ ▶ Replay ] [ ⤓ Export ]      |
|                              |  (2)                            |
|                              |                                 |
|                              |  +-- INPUTS ------------------+  |
|                              |  | Trigger:                   │  |
|                              |  | Buyer message at 13:50:14 │  |
|                              |  | NZST — "Is this still      │  |
|                              |  | available?"               │  |
|                              |  +-----------------------------+ |
|                              |                                 |
|                              |  +-- DECISION CHAIN (3) ------+  |
|                              |  | 1. Memory loaded:          │  |
|                              |  |    • inventory (cached)    │  |
|                              |  |    • pricing rules          │  |
|                              |  |    [hash 4f3a...]          │  |
|                              |  | 2. Skill: marketplace-    │  |
|                              |  |    reply-quote v3 (trust  │  |
|                              |  |    92%)                    │  |
|                              |  | 3. Plan generated:         │  |
|                              |  |    "Yes still avail, $90  │  |
|                              |  |    firm" + 2 alts          │  |
|                              |  | 4. Plan selected by score │  |
|                              |  | 5. Policy: marketplace-   │  |
|                              |  |    reply-needs-ask v2.1   │  |
|                              |  |    → ASK (user OK'd)      │  |
|                              |  | 6. Approval received via  │  |
|                              |  |    Slack (Sam Harlow,     │  |
|                              |  |    14:32:08 NZST)         │  |
|                              |  | 7. Execution: Browser     │  |
|                              |  |    executor on FB MP       │  |
|                              |  | 8. Outcome: sent, 200 OK │  |
|                              |  +-----------------------------+ |
|                              |                                 |
|                              |  +-- PROVENANCE (4) ----------+  |
|                              |  | Model:  claude-opus-4-7   │  |
|                              |  | Prompt: tpl-mr-quote-v3   │  |
|                              |  |   [hash 9a2c...]          │  |
|                              |  | Skill:  marketplace-      │  |
|                              |  |   reply-quote v3.0.2      │  |
|                              |  | Memory: snap_a4f7         │  |
|                              |  | Policy: mr-reply-ask v2.1 │  |
|                              |  | All signed. [Copy hash chain]│ |
|                              |  +-----------------------------+ |
|                              |                                 |
|                              |  +-- OUTPUT (5) --------------+  |
|                              |  | "Yes! Still available at  │  |
|                              |  | $90 firm. Want me to hold │  |
|                              |  | it for pickup this        │  |
|                              |  | weekend?"                  │  |
|                              |  |                            │  |
|                              |  | Sent to: buyer #4821       │  |
|                              |  | Delivered at: 14:32:11 NZST│  |
|                              |  | Buyer response received:  │  |
|                              |  | 14:38:02 (link)            │  |
|                              |  +-----------------------------+ |
+----------------------------------------------------------------+
```

## Annotations — Action drill-in

**(1)** Summary card. Hero of the drill-in. Worker avatar
+ verb in plain English, approver + channel + timestamp,
action ID with copy button. Distinguishing border (subtle
accent tint).

**(2)** Action row. Two buttons:
- `[ ▶ Replay ]` primary button. Routes to
  `/audit/action/<id>/replay`.
- `[ ⤓ Export ]` secondary button. Single-action evidence
  bundle. Opens download immediately (no dialog for
  single-action; bundle defaults to JSON+PDF signed).

**(3)** Decision chain card. The 8-step trace. Each step
is one line. Specific data references (hashes, version
numbers) shown inline. Long-form data hidden behind
expandable rows.

**(4)** Provenance card. All identifying hashes for
reproducibility. Each field is copy-on-click. "Copy hash
chain" copies a single canonical string suitable for
auditor cross-referencing.

**(5)** Output card. The final artifact — the message
text that was sent. Plus delivery confirmation +
downstream consequence (any buyer reply, etc.).

For computer-use actions, also includes a screenshot
sequence at the bottom (collapsed by default) — the
visual record of what the worker saw at decision time.

---

## Layout — Replay sandbox (default state, desktop)

```
+----------------------------------------------------------------+
|                              |  Audit › ... › Replay           |
|                              |                                 |
|                              |  ╔═══════════════════════════╗ |
|                              |  ║ Replaying act_01H9Z7      ║ |
|                              |  ║ Sandbox — no side effects ║ |
|                              |  ║              [● Running…] ║ |
|                              |  ╚═══════════════════════════╝ |
|                              |  (1) HEADER                     |
|                              |                                 |
|                              |  Verdict:  ✓ Reproducible       |
|                              |  (or)      ⚠ Drifted-aligned    |
|                              |  (or)      ⨯ Drifted-misaligned │
|                              |  (2)                            |
|                              |                                 |
|                              |  +-- DIFF (3) ----------------+  |
|                              |  | OUTPUT                      │  |
|                              |  | ── original ──────────────  │  |
|                              |  | "Yes! Still available at   │  |
|                              |  |  $90 firm. Want me to ..."  │  |
|                              |  | ── replayed ──────────────  │  |
|                              |  | "Yes — still available,    │  |
|                              |  |  $90 firm. Want to set up  │  |
|                              |  |  pickup this weekend?"     │  |
|                              |  |                              │  |
|                              |  | (subtle voice shift but    │  |
|                              |  |  policy-equivalent)         │  |
|                              |  +-----------------------------+ |
|                              |                                 |
|                              |  +-- PROVENANCE DIFF (4) -----+  |
|                              |  | Model:    same             │  |
|                              |  | Prompt:   same             │  |
|                              |  | Skill:    v3.0.2 → v3.0.4 │  |
|                              |  |   (updated 2 days ago)     │  |
|                              |  | Memory:   same             │  |
|                              |  | Policy:   same             │  |
|                              |  +-----------------------------+ |
|                              |                                 |
|                              |  [ ↑ Back to action ]           |
|                              |  [ ⤓ Export replay artifact ]   |
+----------------------------------------------------------------+
```

## Annotations — Replay sandbox

**(1)** Replay header. Distinguishing tint (slight
accent) + explicit "Sandbox — no side effects" banner.
Loading state while replay runs (~3-15s depending on
action complexity).

**(2)** Verdict pill. Three states:
- `✓ Reproducible` — same output + same provenance.
  Green tone.
- `⚠ Drifted-aligned` — different output but new output
  also passes policy. Warning tone.
- `⨯ Drifted-misaligned` — different output, new output
  would fail policy. Destructive tone — escalates to
  operator notification too.

**(3)** Output diff. Side-by-side or stacked depending
on viewport width. Text diff highlighting per token
(green for additions, red for removals).

**(4)** Provenance diff. Each axis (model / prompt /
skill / memory / policy) shows `same` or the version
change inline. Click any axis to see deeper context.

Failure modes:
- **Replay unavailable** — if the original model is
  decommissioned. Card replaces verdict with: "Replay
  unavailable — claude-opus-4-7 is no longer deployable
  in the sandbox. Original output preserved below."
- **Computer-use replay** — runs in a virtual display.
  Renders the screenshot sequence + click trace. The
  original sequence and replayed sequence sit side-by-
  side.
- **Side-effect leak detected** — if the sandbox
  somehow ATTEMPTED a real side effect (bug — should
  never happen), the replay aborts with a red banner
  and a self-audit row gets written.

---

## States

### Loading state (activity feed)

Skeleton rows in the timeline (~10 of them). Filters and
search disabled until first paint loads.

### Loading state (drill-in)

Skeleton hero + 4 skeleton cards. ~200ms target P50.

### Loading state (replay)

Full-screen progress: "Replaying… current step: <X of N>"
with a progress bar. Steps include: load original input,
load model, run plan, run policy, compare outcomes.

### Empty state — activity feed (filters return nothing)

Title: `"No actions match"`.
Body: dynamic based on the filter — e.g.
`"Nothing happened in the last hour matching those
filters. Try widening the time range."`.
CTA: `[ Clear filters ]`.

### Empty state — activity feed (no actions ever, fresh install)

Title: `"Nothing's happened yet."`.
Body: `"Once your teammates take their first actions,
they'll appear here."`.

### Error state — activity feed (fetch failed)

Same dashed-border card as approvals-inbox. Title:
`"Couldn't load audit timeline"`. Body: error message.
CTA: `[ Try again ]`.

### Error state — drill-in (action not found)

Title: `"Action not found"`.
Body: `"The action '${id}' doesn't exist or you don't
have permission to view it."`.
CTA: `[ Back to timeline ]`.

### Error state — replay (sandbox failure)

Title: `"Replay failed"`.
Body: error from the sandbox + a "report this" link.
CTA: `[ Back to action ]`.

### Permission-denied state

If user's RBAC role grants read but not unredacted view:
PII in the output and inputs is masked. A small banner
above the drill-in says: `"PII redacted — your role
shows masked customer data."` plus, if applicable, a
`[ Request elevated view ]` link.

---

## Export flow

The `[ ⤓ Export ]` button on the activity feed opens
EvidenceBundleExporter:

```
+-----------------------------+
| Export evidence bundle       |
|                             |
| Scope:                      |
| ( ) Current filtered view   |
| (•) Specific time range     |
| ( ) Sampled audit selection │
|                             |
| Time range:                 |
| From: [ 2026-01-01 ]        |
| To:   [ 2026-03-31 ]        |
|                             |
| Format:                     |
| (•) SOC2 Type II            |
| ( ) GDPR Article 15         |
| ( ) HIPAA disclosure log    |
| ( ) Generic JSON + PDF      |
|                             |
| Tenant: workspace-01-acmeco │
| Includes: 1,247 actions     │
| Est. size: ~4 MB            │
|                             |
| [ Cancel ]  [ ⤓ Export ]    |
+-----------------------------+
```

Format radio controls which schema is exported. SOC2
produces a multi-file zip with attestation-ready schema;
GDPR produces a per-user redaction-aware bundle; HIPAA
produces an access log; Generic produces a documented
JSON+PDF pair.

Click Export → bundle is generated server-side, signed,
delivered via download (large bundles may queue with a
"we'll email you when ready" path for > 100MB).

Each export is itself audited (a meta-audit row records
who exported what, when).

---

## Mobile layout (< 768px)

- Side rail collapses.
- Activity feed timeline takes full width. Rows reflow
  vertically: timestamp + verb on one line, worker +
  outcome + ID on the next.
- Filter / time / export buttons become a single
  scrolling action bar.
- Drill-in: hero + cards stack 1-up. Decision chain
  becomes an accordion (collapsed by default).
- Replay sandbox: diff goes vertical (stacked, not side-
  by-side).
- Export dialog: full-screen modal.

For auditors specifically, mobile is rare — they live
on desktop. But the surface remains functional on
phone for incident response.

---

## Interactions + flows

### Click a row → drill-in

Click anywhere on an ActivityRow → routes to
`/audit/action/<id>`. State preserved in URL (filters
remain, breadcrumb shows). Back button returns to the
filtered feed.

### Drill-in → Replay

`[ ▶ Replay ]` button → routes to
`/audit/action/<id>/replay`. Replay starts immediately.

### Drill-in → Single-action export

`[ ⤓ Export ]` button → single-action bundle download
starts immediately (no dialog).

### Bulk-export

Activity feed → `[ ⤓ Export ]` → dialog with scope
options.

### Cross-system jump

When the drill-in's output is "called Stripe API", a
small "View in Stripe" link appears. Same for Slack,
Google Calendar, GitHub, etc. (configurable per
connector.) Out of v1; revisit for v1.5.

### Hash chain verification

Provenance block has a `[ Copy hash chain ]` button. The
copied string is a JSON payload an external verifier
can use to confirm the action's audit integrity. The
verifier ships as a downloadable script.

---

## Empty + error copy (exact strings, locked)

| Surface | State | Strings |
|---|---|---|
| Activity feed | empty (filters no match) | Title: `"No actions match"` · Body: dynamic per filter · CTA: `"Clear filters"` |
| Activity feed | empty (no actions ever) | Title: `"Nothing's happened yet."` · Body: `"Once your teammates take their first actions, they'll appear here."` |
| Activity feed | error | Title: `"Couldn't load audit timeline"` · Body: `${error}` · CTA: `"Try again"` |
| Drill-in | not found | Title: `"Action not found"` · Body: `"The action '${id}' doesn't exist or you don't have permission to view it."` · CTA: `"Back to timeline"` |
| Drill-in | PII redacted notice | Body: `"PII redacted — your role shows masked customer data."` · CTA (if applicable): `"Request elevated view"` |
| Replay | starting | `"Replaying… current step: ${current} of ${total}"` |
| Replay | unavailable (model decom'd) | Title: `"Replay unavailable"` · Body: `"${modelName} is no longer deployable in the sandbox. Original output preserved below."` |
| Replay | side-effect leak | Title: `"Replay aborted — side-effect leak detected"` · Body: `"This should never happen. A self-audit row has been written. Please report to operations."` |
| Export | size warning | `"This export is ~${size}. Continue?"` (for > 50MB) |
| Export | queued | `"Your export is being prepared. We'll email you when it's ready (~${eta})."` |

---

## Implementation notes for Track E

Track E brief slicing:

- **E5-a: Audit timeline — route + activity feed.**
  Route at `/audit`, server-paginated timeline, ActivityRow
  component, filter dropdown skeleton, search input.
- **E5-b: Audit timeline — filter popover.** Multi-
  dimension filter UI (worker, channel, action type,
  status, approver, cost band, data classification).
  URL state.
- **E5-c: Audit timeline — drill-in route.** Route at
  `/audit/action/<id>`, hero + decision chain +
  provenance + output cards.
- **E5-d: Audit timeline — replay sandbox.** Route at
  `/audit/action/<id>/replay`, run replay, render
  verdict + diff + provenance diff.
- **E5-e: Audit timeline — evidence bundle exporter.**
  Single-action and bulk export dialog. Format selector.
  Generated via server-side render of audit data into
  schema-correct bundles.
- **E5-f: Audit timeline — hash chain verifier (CLI).**
  Standalone Node CLI that verifies a bundle's hash
  chain. Distributed as `npm install -g @skycot/verify`.
  Standards-aligned.
- **E5-g: Audit timeline — RBAC redaction layer.** Cross-
  cutting concern — PII redaction at render time based on
  the viewer's RBAC. Applies to all cards.
- **E5-h: Audit timeline — cross-system jump links.**
  Per-connector deep-link generators (Stripe, Slack,
  etc.). Deferred to v1.5.

8 briefs total. E5-a + E5-c are large (~400 LOC each);
others are smaller. E5-h deferred.

New primitives:
- `ActivityRow` (composite, ~150 LOC, shared with worker
  profile Activity tab).
- `FilterBar` (~200 LOC, generic; also usable on
  approvals inbox).
- `ProvenanceBlock` (~100 LOC).
- `ReplayDiff` (~250 LOC).
- `EvidenceBundleExporter` (~350 LOC).

---

## Cross-references

- Operator console "Audit" section (planned for Track
  E's operator-console slice if scoped): see
  [operator.md](../operator.md). The Audit timeline IS
  the operator's audit surface — the operator console
  doesn't duplicate it, it links into it.
- Worker profile Activity tab: see
  [wireframe-worker-profile.md](wireframe-worker-profile.md).
  Worker-scoped slice of the same timeline.
- Approval card "Why?" expansion: see
  [wireframe-approvals-inbox.md](wireframe-approvals-inbox.md).
  Reads from the same audit data; links here for full
  context.

---

## Open questions

`(needs validation)`

- Are 8 filter dimensions too many? Do auditors use them
  all or do most queries fit 2-3 dimensions?
- Is server-side pagination right (50/page) or should
  we go cursor-based with infinite scroll?
- Should drill-in always show full provenance, or
  hide it behind "Show technical details" for non-
  technical viewers?
- Should the replay verdict trichotomy be more
  granular? Auditors might want 5+ verdict states.
- Is single-action export-immediate-download right, or
  should we always force the dialog for an opt-in
  confirmation?
- Should we cache replay results (cache key = action_id
  + current_state hash) to avoid re-running expensive
  replays? Memory cost vs CPU cost tradeoff.
- Should evidence bundles be encrypted at rest? Trade-
  off between auditor ease-of-use and information-
  security posture.

Plan to validate by:
1. Running a mock SOC2 attestation with synthetic data
   pre-launch.
2. Engaging a SOC2 audit firm to read the SOC2 format
   pre-customer-shipping.
3. Stress-testing the activity feed at 1M rows
   (pagination behavior).
4. Shadow-running replay on 1000 historical actions
   pre-launch and reading the verdict distribution.

---

**File status**: D5 — authored 2026-05-11 as the canonical
spec for the audit-timeline surface. Updated alongside any
PR that touches the surface.
