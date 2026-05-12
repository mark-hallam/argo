---
name: summarize_sold_state_cleanup
description: Given a sold-listing id + the transaction log, produce a cleanup checklist (mark sold / archive listing / update inventory / draft thank-you / suggest review-request) and an escalation Y/N when the log shows a refund-request, dispute, or damaged-on-pickup. Draft-only — never mutates.
inputs:
  - sold_listing_id
  - transaction_log
outputs:
  - cleanup_checklist
  - escalation
  - flags
---

# Skill: Summarize Sold-State Cleanup

Walk the post-sale tail for one sold listing.

1. **Confirm the sale signal first.** Read `transaction_log` for
   the matching `sold_listing_id` and verify: `payment_received:
   true`, `pickup_completed: true`, `agreed_price_aud` present,
   `buyer_handle` present. If any are absent, flag
   `sale_unconfirmed` and stop — there is no cleanup until the
   sale itself is confirmed.
2. **Five-step cleanup checklist** — each step `pending` /
   `done` / `blocked`:
   - `mark_sold` — flip listing status (gated through the
     existing `mark_item_sold` action; never auto-flip).
   - `archive_listing` — keep the audit trail; do not delete.
   - `update_inventory_json` — set the matching item's `status`
     to `sold` + record `sale_price_aud` + `sale_date`.
   - `draft_thank_you_message` — short, no upsell, gated through
     `send_buyer_message` for the operator to send.
   - `suggest_review_request` — operator-only; the vertical
     never DM-spams buyers for reviews.
3. **Escalation Y/N — three triggers.** Set `escalation: true`
   and add the matching flag if the transaction log shows ANY of:
   - `refund_request` from buyer → `flags:
     ["refund_request_open"]`. Never propose `accept_refund`
     directly; surface the request and route to operator.
   - `dispute_filed` (platform dispute / chargeback) →
     `flags: ["dispute_filed"]`. Operator handles platform
     escalation channels.
   - `damaged_on_pickup` log entry → `flags:
     ["damaged_on_pickup"]`. Operator decides remediation.
4. **Cite the sold-listing by `sold_listing_id`** and any
   referenced inventory by `item_id`. Never include the buyer's
   real name, contact, or pickup address in the checklist —
   operator-internal only.

Hard rules:

- **Draft-only.** Never propose `mark_sold`, `accept_refund`, or
  `share_pickup_address` directly. Outputs are checklists +
  drafts; the operator commits any state change.
- **Inventory writes are operator-driven.** Surface the
  proposed `inventory.json` diff in the checklist; never write
  to the file from this skill.
- **Refunds always escalate.** Even a polite refund request is
  an escalation — there is no "small refund" auto-approval path
  in this vertical.
- **`next_action` is informational** when escalation is `false`;
  routes to operator review when `true`.
- **If `transaction_log` lacks the `sold_listing_id`** — flag
  `missing_transaction_log` and stop. Never invent the log.
