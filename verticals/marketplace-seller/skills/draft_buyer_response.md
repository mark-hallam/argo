---
name: draft_buyer_response
description: Given a buyer's incoming message + the listing context + inventory entry, produce three tone variants (friendly / informative / firm-on-price), a pickup-logistics template, and an escalation Y/N when the buyer asks for off-platform contact, demands aggressive haggling, or hints at a dispute. Draft-only — never sends.
inputs:
  - buyer_message
  - listing_context
  - inventory_entry
outputs:
  - tone_variants
  - pickup_logistics_template
  - escalation
  - flags
---

# Skill: Draft Buyer Response

Produce three tone variants of a reply to one buyer message.

1. **Read the buyer message + listing context first.** Identify
   the buyer's ask: question (`spec` / `condition` / `pickup` /
   `price`), intent (`browsing` / `committed`), and tone
   (`polite` / `terse` / `pushy`). Cite the buyer-message line
   that anchors each tag — never infer attributes not in the
   message.
2. **Three tone variants — same content, different posture.**
   - `friendly`: warm ack, full answer, soft pickup next step (~80 words).
   - `informative`: terse, fact-first, link to listing (~50 words).
   - `firm_on_price`: polite but anchors price floor from
     `inventory_entry.target_price_aud`; declines counter-offers
     below the floor without negotiating (~60 words).
   All three pull facts from `listing_context` + `inventory_entry`
   only. Never invent specs, condition claims, or pickup zones.
3. **Pickup-logistics template.** Standard wording: pickup zone
   (city/suburb only — never the operator's street address),
   pickup-window options (`weekday_evening` / `weekend_morning`),
   payment-on-pickup (cash or instant transfer). Placeholder
   only; operator fills in concrete time + address before sending.
4. **Escalation Y/N — three triggers.** Set `escalation: true`
   and add the matching flag if ANY of:
   - Off-platform contact request (WhatsApp, SMS, personal
     email, "let's take this to text") → `flags:
     ["off_platform_contact_request"]`.
   - Counter-offer below the price-floor by ≥ 30%
     (`floor = target_price_aud × 0.7`) → `flags:
     ["below_price_floor_30pct"]`.
   - Damage / refund / dispute hints ("broken", "refund", "not
     as described", "fraud") → `flags:
     ["damaged_or_dispute_signal"]`.

Hard rules:

- **Draft-only.** Never propose `send_buyer_message` or
  `share_pickup_address` directly. The outputs are drafts; the
  operator approves any send.
- **No fabricated condition claims.** If the buyer asks about
  bearings, wheel-wear, or impact damage and the inventory entry
  is silent, surface "unknown — operator confirm" rather than
  guessing. Consumer-honesty (FTC-style "no false claims")
  applies.
- **Never share private contact info.** No phone, email, or
  street address in any variant — pickup-logistics is a city
  zone + window, not coordinates.
- **`next_action` is informational** when escalation is `false`;
  routes to operator review when `true`. The vertical never
  auto-replies.
