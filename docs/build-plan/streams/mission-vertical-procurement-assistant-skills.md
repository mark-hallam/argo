# Mission load-bearing wire — Procurement-Assistant vertical: 2+ skills to reach 6

**Owner**: claude. **Depends on**: `verticals/procurement-assistant/` (4 existing skills).
**Authored against**: origin/main `281ba83d` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-procurement-assistant-skills`.

> SOURCE: MISSION_BACKLOG.md Section 4 #137. procurement-assistant ships with 4 skills (compare_bids / contract_review / draft_rfq / vendor_health_check). Brief brings to 6.

## Scope

2 NEW skill files:

1. **`track_renewal_calendar.skill.md`** — given the operator's active vendor list + contract end dates (from `memory.vendors[]`), output a 90-day-lookahead renewal report: contracts ending soon, current spend per contract, recommended action (renew-as-is / renegotiate / let-lapse / consolidate-with-X). Output JSON + Markdown summary. Internal report; renewal decisions approval-gated downstream.
2. **`detect_invoice_anomaly.skill.md`** — given a vendor invoice + the contract terms, flag discrepancies: pricing drift (>5% from agreed unit price), unbilled discounts, late-fee additions, scope expansion. Output JSON: `{ anomalies: [{type, expected, actual, severity, evidence_clause}], recommended_action: pay_as_is | dispute | request_clarification | hold_for_legal }`. Approval-gated before any disputed-amount communication.

## Files you may touch

- `verticals/procurement-assistant/skills/track_renewal_calendar.skill.md` (NEW, ≤ 90 LOC)
- `verticals/procurement-assistant/skills/detect_invoice_anomaly.skill.md` (NEW, ≤ 90 LOC)

## Files you must NOT touch

- Existing 4 skill files; `vertical.toml`; prompts.

## Acceptance criteria

- [ ] 2 new `.skill.md` files with full frontmatter + sections
- [ ] `track_renewal_calendar` 90-day window + 4-action vocab
- [ ] `detect_invoice_anomaly` 4-anomaly-type list + 4-action vocab + 5% threshold explicit
- [ ] Both reference contract terms by citation (no fabrication)
- [ ] No file > 100 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-vertical-procurement-assistant-skills`
- PR title: `verticals: 2 skills for procurement-assistant (mission-vertical-procurement-assistant-skills)`

## Notes

- Invoice anomaly detection is the highest-leverage cost-recovery skill — small overcharges add up. The 5% threshold is operator-tunable but the default is the documented recommendation.
- Don't claim contract terms the skill can't cite. If the contract says "pricing reviewed annually" and the invoice claims a mid-year increase, the skill flags the gap; it doesn't argue the legal interpretation.
