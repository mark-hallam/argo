---
name: triage_inbox
description: Bucket each unread email thread into action_required, info_only, spam_or_promo, or respond_with_template; never sends or auto-replies.
inputs:
  - thread_id
  - sender
  - subject
  - snippet
  - received_at
outputs:
  - bucket
  - rationale
  - draft_reply
  - escalation_required
---

# Skill: Triage Inbox

Read each unread thread and emit one of four buckets. Output is
advisory; the principal reviews every drafted reply before sending.

1. **`action_required`** — needs a decision, a reply, or a calendar
   change from the principal. Surface immediately with a 1-line
   "what they want" summary and the implicit deadline.
2. **`info_only`** — FYI / newsletter from a high-signal source / status
   update with no required reply. Batch into the daily digest.
3. **`spam_or_promo`** — cold outbound, vendor marketing, mailing-list
   noise. Recommend archive; never auto-archive.
4. **`respond_with_template`** — high-volume canned replies the
   principal has approved patterns for (intro requests, calendar
   declines, "I'll have my EA reach out"). Generate a draft reply
   matched to the principal's voice from `drafted_replies` memory.

Always set `escalation_required = true` for any thread matching:

- Legal / regulator / outside counsel.
- Financial: wires, invoices, equity, payroll, tax IDs, account
  changes, expense over auto-approve threshold.
- Family emergency: hospital / school / urgent / `frequent_contacts.family`.
- HR-sensitive: termination, performance, harassment, EAP.
- Press / journalist / IPO / M&A-adjacent.

For escalated threads, leave `draft_reply` empty. The principal
handles these unmediated.

Output shape:

```json
{
  "bucket": "action_required",
  "rationale": "Direct ask from board chair — needs response within 24h.",
  "draft_reply": "",
  "escalation_required": false
}
```
