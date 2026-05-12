# Mission load-bearing wire — Marketplace-Seller vertical: 2+ skills to reach 6

**Owner**: claude. **Depends on**: `verticals/marketplace-seller/` (4 existing skills).
**Authored against**: origin/main `281ba83d` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-marketplace-seller-skills`.

> SOURCE: MISSION_BACKLOG.md Section 4 #137. marketplace-seller ships with 4 skills (audit_inventory_readiness / draft_buyer_response / post_listing / summarize_sold_state_cleanup). Brief brings to 6. Sibling `marketplace-seller-browser` vertical handles browser-action variants.

## Scope

2 NEW skill files:

1. **`reprice_stale_listing.skill.md`** — given a listing that hasn't sold within an operator-defined staleness threshold (default 14 days), propose 3 reprice options: aggressive (−15%), moderate (−7%), minimal (−3%). Output JSON: `{ current_price, days_listed, reprice_options: [{level, new_price, expected_sale_lift_band}], no_change_rationale, requires_approval: true }`. Each option includes evidence (similar-listing data if available) or `assumption: generic-elasticity`.
2. **`negotiate_offer.skill.md`** — given an incoming buyer offer + the listing's floor price (`memory.listing_floors[listing_id]`), draft a measured response: accept / counter-with-justification / decline-politely. Output Markdown + structured `{ action, counter_price?, rationale }`. Approval-gated. Never accepts an offer below floor.

## Files you may touch

- `verticals/marketplace-seller/skills/reprice_stale_listing.skill.md` (NEW, ≤ 90 LOC)
- `verticals/marketplace-seller/skills/negotiate_offer.skill.md` (NEW, ≤ 90 LOC)

## Files you must NOT touch

- Existing 4 skill files; `vertical.toml`; prompts; `verticals/marketplace-seller-browser/` (sibling).

## Acceptance criteria

- [ ] 2 new `.skill.md` files with full frontmatter + sections
- [ ] `reprice_stale_listing` 3-option vocab with explicit percentages + evidence-or-assumption marker
- [ ] `negotiate_offer` enforces floor-price respect (never auto-accept below)
- [ ] Both approval-gated
- [ ] No file > 100 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-vertical-marketplace-seller-skills`
- PR title: `verticals: 2 skills for marketplace-seller (mission-vertical-marketplace-seller-skills)`

## Notes

- The −15/−7/−3 reprice band is operator-tunable but the defaults are the load-bearing recommendation. Don't drift them across the skill set.
- `negotiate_offer` floor-price respect is a hard rule — accepting below floor breaks the operator's economics. Document it explicitly.
