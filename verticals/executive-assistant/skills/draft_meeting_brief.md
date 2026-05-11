---
name: draft_meeting_brief
description: Given a calendar invite, produce a 1-page brief with attendees, relevant prior threads, agenda, and open questions for the principal.
inputs:
  - event_id
  - subject
  - attendees
  - start_time
  - description
outputs:
  - brief
---

# Skill: Draft Meeting Brief

For each upcoming meeting on the principal's calendar, assemble a
1-page brief the principal can skim in under 60 seconds.

1. **Attendees** — name, title, company, and a 1-line "who they are
   to us" note (from `frequent_contacts` memory or open-web research).
   Flag any new face the principal hasn't met before.
2. **Relevant prior threads** — pull the most recent 2–3 email threads
   matching the attendee set OR the meeting subject. One-line summary
   per thread with the last decision recorded.
3. **Agenda** — if the invite carries an explicit agenda, use it. If
   not, infer 3–5 likely topics from the subject + prior threads and
   label them as inferred.
4. **Open questions** — surface decisions the principal needs to come
   ready to make, and any context the principal owes the room
   (commitments made in prior threads, follow-ups promised).
5. **Logistics** — location, dial-in, prep materials linked in the
   invite. Flag if the meeting is back-to-back with another (no buffer).

Never invent attendee titles, company affiliations, or prior-thread
content. If a field is unknown, say so.

Output shape:

```json
{
  "brief": {
    "attendees": [{"name": "...", "title": "...", "company": "...", "note": "..."}],
    "prior_threads": [{"subject": "...", "last_decision": "...", "date": "..."}],
    "agenda": ["..."],
    "agenda_source": "explicit_invite | inferred",
    "open_questions": ["..."],
    "logistics": {"location": "...", "dial_in": "...", "back_to_back": false}
  }
}
```
