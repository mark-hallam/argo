# Mission load-bearing wire — marketplace-seller vertical flesh-out

**Owner**: claude. **Depends on**: existing `verticals/marketplace-seller/` (has `vertical.toml` + 1 skill `post_listing.md` + `memory/` + `auto_approval.toml` 4-line + `inventory.json` + `eval/seed.jsonl` 4 fixtures); `verticals/recruiter-screener/` (#1340) + `verticals/marketplace-seller-browser/` as canonical templates.
**Authored against**: origin/main `d20fabc3` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-marketplace-seller-flesh-out`.

> SOURCE: MISSION_BACKLOG.md Section 4 Claim 11 #131 ("Charter seed for marketplace-seller"). **Tier A (vertical).** Runs a small marketplace-selling workflow (inline-skates demo from the original charter) — inventory review, listing post, buyer follow-up, sold-state cleanup. Stub currently has 1 skill (`post_listing`); this brief adds 3 more skills + expands eval ≥ 9 + expands auto_approval to the full pattern. Drafts-only — every public/buyer-touching action requires operator approval. Consumer-marketplace honesty load-bearing (FTC-style "no false claims" + photo-truth rules).

## Scope

EXTEND content-only `verticals/marketplace-seller/` (add 3 skills + expand eval + expand auto_approval, ≤ 400 LOC delta — pattern budget: 3 NEW skills × ≤ 65, eval/seed.jsonl ≥ 9 fixtures, auto_approval ≤ 25):

- KEEP existing `vertical.toml` (169 LOC — already substantive), `prompts/system.md`, `prompts/persona.md`, `inventory.json`, `memory/`, existing skill `skills/post_listing.md`, existing 4 eval fixtures
- 3 NEW skills (drafts-only):
  - `audit_inventory_readiness` — inventory.json entry in; missing-fields checklist (photo-coverage / size-label / soulplate-photo / wheel-photo / pricing-anchor) + suggested-price-band-w-rationale (using cached comparable-listings) + listing-readiness-Y/N out
  - `draft_buyer_response` — buyer message + listing context + inventory.json in; suggested-response (3 tone variants: friendly / informative / firm-on-price) + escalation-Y/N (if buyer asks for off-platform contact / outside-pickup-zone / haggles below price-floor) + pickup-logistics-template out
  - `summarize_sold_state_cleanup` — sold-listing-id + transaction-log in; suggested cleanup-checklist (mark sold / archive listing / update inventory.json / draft thank-you-message / suggest review-request) + escalation-Y/N (refund-request / dispute / damaged-on-pickup) out
- KEEP existing skill `post_listing.md` (already covers listing draft pattern)
- EXPAND `auto_approval.toml` (currently 4 lines) to the canonical 25-LOC wake-list pattern: wake-listed skills include `audit_inventory_readiness`, `draft_buyer_response`, `summarize_sold_state_cleanup`, and `post_listing`-as-draft (the existing skill); `post_listing_public` (the action that pushes to the marketplace), `send_buyer_message`, `mark_sold`, `share_pickup_address`, `accept_refund` intentionally absent + gated through `[escalation].human_approval_required`
- EXPAND `eval/seed.jsonl` from 4 → ≥ 9 fixtures: 1 audit-inventory-ready, 1 audit-inventory-missing-photos, 1 draft-buyer-friendly-tone, 1 draft-buyer-firm-on-price, 1 summarize-sold-cleanup-clean, 1 summarize-sold-cleanup-refund-escalation, 1 retain-existing post_listing happy, 3 escalation (buyer-asks-off-platform-contact / buyer-below-price-floor-by-30pct / damaged-on-pickup-dispute)

NO Rust. NO migrations. NO crate changes. Pattern is identical to existing verticals — re-use shapes 1:1.

This is NOT in scope: actual marketplace-API connectors (eBay / Mercari / Facebook Marketplace — separate; the existing E2B-based `marketplace-seller-browser` sibling already handles browser-based posting); shipping-rate calculation; payment-processing wiring (consumer-marketplace pay-on-pickup is the assumed model); review/feedback management (separate skill if needed later); image-quality-detection model (operator validates photo-coverage manually); price-floor enforcement at sale-time (the vertical surfaces price-floor breaches; doesn't block sales).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `verticals/marketplace-seller/skills/audit_inventory_readiness.md` (NEW, ≤ 65 LOC)
- `verticals/marketplace-seller/skills/draft_buyer_response.md` (NEW, ≤ 65 LOC)
- `verticals/marketplace-seller/skills/summarize_sold_state_cleanup.md` (NEW, ≤ 65 LOC)
- `verticals/marketplace-seller/auto_approval.toml` (EXPAND existing 4-line → ≤ 25 LOC canonical pattern)
- `verticals/marketplace-seller/eval/seed.jsonl` (EXPAND existing 4-fixture file to ≥ 9 fixtures)
- `verticals/marketplace-seller/vertical.toml` (EXTEND only — add 3 `[actions.<skill>]` blocks for the 3 NEW skills; do NOT rewrite the existing 169 LOC)

## Files you must NOT touch

- Any `crates/**` Rust source
- `crates/argo-store/migrations/**`
- Any other `verticals/<other>/**` directory (incl. sibling `marketplace-seller-browser/`)
- `verticals/marketplace-seller/prompts/{system,persona}.md`
- `verticals/marketplace-seller/skills/post_listing.md` (existing skill stands)
- `verticals/marketplace-seller/memory/` (existing schema stands)
- `verticals/marketplace-seller/inventory.json` (existing inventory stands)

## Acceptance criteria

- [ ] `argo vertical list` continues to show `marketplace-seller <version>` (version may bump 0.1.0 → 0.2.0)
- [ ] `argo vertical lint marketplace-seller` exits 0 errors (baseline warns acceptable)
- [ ] `argo vertical show marketplace-seller --json` returns valid envelope (id / name / version / persona / skill_count = 4 / eval_seed_count ≥ 9)
- [ ] All 3 NEW skill files have YAML frontmatter (`name` + `description` + `inputs` + `outputs`)
- [ ] `eval/seed.jsonl` has ≥ 9 fixtures total, ≥ 3 of which are escalation cases
- [ ] `auto_approval.toml` wake list is the canonical pattern (not the 4-line stub); `post_listing_public` / `send_buyer_message` / `mark_sold` / `share_pickup_address` / `accept_refund` absent + gated through `[escalation].human_approval_required`
- [ ] No Rust changes; no migrations
- [ ] Every new file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vertical-marketplace-seller-flesh-out`
- PR title: `vertical: marketplace-seller flesh-out (+3 skills + ≥9 eval + canonical auto_approval) (mission-vertical-marketplace-seller-flesh-out)`

## Notes for the implementer

- The marketplace-seller stub is the original charter demo (inline-skates) — do NOT change the domain framing. The 3 new skills slot alongside `post_listing` to cover the full sell-cycle (audit → list → buyer-message → sold-cleanup).
- Treat `verticals/recruiter-screener/` (#1340) as the canonical pattern for the 4-skill / canonical-auto-approval / ≥ 9-eval shape; treat `verticals/marketplace-seller-browser/` as the sibling that handles the actual browser-driven posting (this vertical drafts; the browser-sibling executes).
- `audit_inventory_readiness` is the "listing-prep" gate: photo-coverage list + comparable-pricing rationale. Operator decides whether to proceed; the vertical never auto-posts.
- `draft_buyer_response` covers the meaty operator concern — three tone variants because buyer relationships vary. Escalation triggers (off-platform contact / aggressive haggling / dispute) prevent the vertical from autonomously committing to anything the operator might regret.
- `summarize_sold_state_cleanup` handles the post-sale tail: archive / inventory update / thank-you draft / review prompt. Escalates on refund / dispute / damaged-on-pickup.
- The existing 4-line `auto_approval.toml` is incomplete — replace it with the canonical wake-list pattern from `recruiter-screener` (which version-bumps the vertical to 0.2.0).
- DO NOT add price-prediction or demand-forecasting features. The vertical surfaces a price-BAND from cached comparables (in memory), not a predicted-sale-velocity.
- The consumer-honesty load-bearing rule: the system prompt (already present) covers "describe condition truthfully" — preserve it. New skills inherit; do not add separate honesty-rules per skill.
