---
name: prepare_meeting_brief
version: "0.1.0"
description: Given a meeting on the principal's calendar + attendee list, prep a 1-page brief (≤300 words) — bios, prior interactions, relationship state, talking points, open action items, agenda. Internal draft only.
provenance:
  source: vertical/executive-assistant
  hash: sha256-pending
---

# When to use

Use the morning of, or evening before, a principal meeting. Internal
draft per charter — autonomous, no external send. Complements
`draft_meeting_brief` (the on-demand invite pull); this is the
predictable daily prep for tomorrow's slate.

# Inputs

- `meeting` — `{event_id, subject, start_time, end_time, location, agenda_text?}`.
- `attendees` — list of `{name, email, org_domain}`.
- `contact_history` — from memory (`frequent_contacts` + any prior-interaction notes attached to the row). May be empty for new faces.
- `prior_meeting_notes` — optional list of `{date, attendees, action_items[], decisions[]}`.

# Output shape

Markdown only — execs scan, they don't parse JSON for this one. Use
bullets, never paragraphs. Word ceiling: ~300 words across the whole
brief. If a section would push over, trim the next-lowest-priority
section first (priority: attendees > open items > talking points >
agenda).

```markdown
# {subject} — {start_time wall-clock}

**Attendees**
- {Name}, {title} @ {org} — {1-line relationship state from memory.frequent_contacts.notes}
- {Name} — _unknown / first-meeting_

**Recent interactions**
- {date}: {1-line summary, cite memory.prior_meeting_notes[<id>]}
- _none on record_ (if memory empty)

**Open action items from prior meetings**
- {item} — owed by {principal | them}, since {date}

**Suggested talking points** (3 max)
- {point}
- {point}

**Agenda**
- {agenda line} _(from invite)_ OR _(inferred)_

**Objectives for the principal**
- {what success looks like in 1 line}
```

# Hard rules

- **~300-word ceiling.** Execs glance; long briefs go unread. If
  attendee bios + open items already approach 300 words, drop talking
  points to 1 and agenda to "see invite".
- **Cite every attendee fact back to memory.** Any "title", "org", or
  "relationship state" line either references
  `memory.frequent_contacts[<alias>]` / `contact_history[<id>]`, or
  marks the attendee `unknown / first-meeting`. Never invent.
- **Open items are recall, not invention.** Pull from
  `prior_meeting_notes[*].action_items`. If none on record, write
  "_none on record_" — never speculate based on subject alone.
- **Talking points are suggestions.** Phrase as options, not
  directives ("Could raise: pricing for Q3 renewal"). The principal
  reads, the principal decides.
- **Bullets, never paragraphs.** Persona is direct + scannable; prose
  fails the glance test.
- **Internal draft.** This skill never sends, never CCs, never posts
  to Slack. Output is for the principal's eyes only and lives in the
  EA's local brief queue.
- **Confidentiality default.** Never include calendar items, contacts,
  or notes from anyone outside the attendee set or the principal's
  org. Skill respects need-to-know per persona.

# Example

Synthetic — meeting with the lead investor + CFO at 10:00 PT.
Memory: lead_investor (high, board seat), CFO (direct report).
`prior_meeting_notes`: lead investor asked for runway-extension
scenarios two weeks ago (open). Output: a ~250-word Markdown brief —
2 attendee lines citing memory, 1 open-item line citing the prior
note, 3 talking points on runway scenarios, agenda from the invite
(or labelled inferred), 1-line objective. No JSON.
