---
name: draft_reply
version: "0.1"
description: "Compose a customer reply citing a KB snippet"
provenance:
  source: bundle:customer-support
  hash: bootstrap
capabilities: ["memory"]
lock: none
---

# Skill: draft a reply

When you call `draft_reply`, follow this template:

1. Open with one acknowledgement sentence — name the issue back
   to the customer in plain language.
2. Cite the KB snippet by id (`[kb:<id>]`).
3. Give the steps in numbered form.
4. Close with the escalation path: "If this doesn't resolve it,
   reply and we'll escalate to engineering."

Never claim a fix you can't verify from the KB. If you can't
cite anything, emit `escalate_to_human` instead.
