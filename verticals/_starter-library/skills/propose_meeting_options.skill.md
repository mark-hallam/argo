---
name: propose_meeting_options
version: "0.1.0"
description: "Drafts a short message proposing 2-3 specific meeting times to an external party, given a list of internally-confirmed slots."
provenance:
  source: vertical/_starter-library
  hash: sha256-pending
---

# When to use

After `find_calendar_overlap` (or any equivalent) has produced
candidate slots and the operator wants to send the external party
a "does any of these work?" message. Use any time the deliverable
is "propose times to <recipient>."

# Inputs

- `slots`: list of candidate intervals, each `[start, end]` in
  ISO-8601 with timezone.
- `recipient_timezone`: IANA timezone string for the external
  party (e.g. `"Europe/London"`). Slots are presented in this zone.
- `meeting_purpose`: one-sentence framing the operator wants the
  recipient to see.
- `tone`: `"formal"`, `"warm"`, `"neutral"`. Default: `"warm"`.

# Output

A short message (2-4 sentences) listing 2-3 slots in the
recipient's local timezone, with day-of-week labels and a single
clear ask ("does any of these work?"). No more than 3 options;
more choices reduce reply rate.

# Examples

- 3 slots, recipient in Europe/London → "Hi — would any of these
  work for <purpose>? Tue 14:00 BST, Wed 10:00 BST, or Thu 16:00
  BST. Happy to flex if not."
- 1 slot supplied → propose the one slot and offer to find
  alternatives if it doesn't suit, rather than padding with
  invented options.

# Approval posture

Drafts the message. Operator sends. Does not auto-send email,
calendar invites, or chat messages.

# Notes

- Always render slots in `recipient_timezone`, not the operator's.
  Include the timezone label inline so the recipient cannot
  misread it.
- Cap at 3 options. If more were supplied, pick the top 3 by
  ranking rules from `find_calendar_overlap`.
- Avoid "let me know your availability" — that puts the work
  back on them and defeats the point of proposing slots.
