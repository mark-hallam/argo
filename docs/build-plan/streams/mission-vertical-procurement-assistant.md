# Mission load-bearing wire — Procurement-assistant vertical

**Owner**: claude. **Depends on**: existing vertical-template scaffold (`verticals/executive-assistant/` as canonical content-only-vertical pattern just merged in #1336).
**Authored against**: origin/main `69d3e28e` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-procurement-assistant`.

> SOURCE: MISSION_BACKLOG.md Section 4 Claim 11 #159 ("Procurement-assistant vertical"). Mirrors the just-merged executive-assistant (#1336) / research-analyst (#1337) shape — content-only vertical, no Rust, no migrations. **Drafts only.** Every "place purchase order" / "sign agreement" / "approve vendor" action gates behind human approval. The system prompt names budget-ceiling as the load-bearing constraint.

## Scope

NEW content-only `verticals/procurement-assistant/` (11 files, ≤ 500 LOC total — pattern budget: system ≤ 120, persona ≤ 35, skills 4×≤60, schema ≤ 35, auto_approval ≤ 25, inventory ≤ 55, vertical.toml ≤ 55, eval/seed.jsonl ≥ 8 fixtures):

- `vertical.toml` — id=`procurement-assistant`, version=`0.1.0`, optional ariba/coupa MCP, `[escalation].human_approval_required = ["place_purchase_order", "sign_agreement", "approve_vendor", "renew_contract"]`
- `prompts/system.md` — budget-ceiling enforcement (every recommendation must cite the budget line item + remaining balance + spend-window), vendor-qualification rules (vendor must be in `approved_vendors` memory or escalate), 3-quote rule for spend ≥ $5k (configurable per tenant in inventory), contract-renewal lookahead (30/60/90 day flag), conflict-of-interest checks, escalation triggers (over-budget / unapproved-vendor / single-bid-over-threshold / sole-source-without-justification / contract-auto-renew-imminent)
- `prompts/persona.md` — Drew, procurement specialist; total-cost-of-ownership over sticker-price; always names trade-offs
- 4 skills:
  - `draft_rfq` — requirement + budget + vendors in; RFQ document with eval criteria + scoring weights out
  - `compare_bids` — bid set in; total-cost-of-ownership comparison + risk-adjusted ranking + recommendation with rationale out
  - `vendor_health_check` — vendor name in; recent-news + financial-health-signals + sanctions-list-status + escalation-flag-if-any out
  - `contract_review` — contract + comparison-position in; key-terms summary + redlines vs `firm_terms` memory + escalation-flag-if-deviation out
- `memory/schema.toml` — `approved_vendors` (vendor_id / category / tier / contact / last_review_date), `budget_lines` (line_id / period / ceiling / used / remaining), `firm_terms` (boilerplate-must-haves), `sanctions_watchlist`
- `eval/seed.jsonl` — 8+ fixtures: 1 RFQ-happy-path, 2 bid-compare (1 clear-winner + 1 close-call), 1 vendor-health-clean, 1 contract-review, 3 escalation (over-budget / unapproved-vendor / sanctions-hit)
- `auto_approval.toml` — wake list includes draft_rfq + compare_bids + vendor_health_check + contract_review only; place_purchase_order / sign_agreement / approve_vendor / renew_contract intentionally absent
- `inventory.json` — sample `approved_vendors` (5 entries across categories), sample `budget_lines` (3 entries), sample `firm_terms` (5 boilerplate clauses), sample `sanctions_watchlist` (synthetic entries — never load real OFAC data into inventory)

NO Rust. NO migrations. NO crate changes. Pattern is identical to executive-assistant / research-analyst — re-use shapes 1:1.

This is NOT in scope: actually placing orders anywhere (ERP connectors separate); supplier-portal integration (Ariba / Coupa connectors are separate MCP briefs); spend-analytics dashboards (separate); contract-management system integration (separate); P-card / corp-card direct integration; live sanctions-list ingestion (this is a separate compliance brief — for v1 the watchlist is a static memory inventory).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `verticals/procurement-assistant/vertical.toml` (NEW, ≤ 55 LOC)
- `verticals/procurement-assistant/prompts/system.md` (NEW, ≤ 120 LOC)
- `verticals/procurement-assistant/prompts/persona.md` (NEW, ≤ 35 LOC)
- `verticals/procurement-assistant/skills/draft_rfq.md` (NEW, ≤ 60 LOC)
- `verticals/procurement-assistant/skills/compare_bids.md` (NEW, ≤ 60 LOC)
- `verticals/procurement-assistant/skills/vendor_health_check.md` (NEW, ≤ 60 LOC)
- `verticals/procurement-assistant/skills/contract_review.md` (NEW, ≤ 60 LOC)
- `verticals/procurement-assistant/memory/schema.toml` (NEW, ≤ 35 LOC)
- `verticals/procurement-assistant/eval/seed.jsonl` (NEW, ≥ 8 fixtures)
- `verticals/procurement-assistant/auto_approval.toml` (NEW, ≤ 25 LOC)
- `verticals/procurement-assistant/inventory.json` (NEW, ≤ 55 LOC)

## Files you must NOT touch

- Any `crates/**` Rust source
- `crates/argo-store/migrations/**`
- Any other `verticals/<other>/**` directory
- Any frontend / web crate

## Acceptance criteria

- [ ] `argo vertical list` includes `procurement-assistant 0.1.0`
- [ ] `argo vertical lint procurement-assistant` exits 0 errors (warns of the executive-assistant / research-analyst baseline shape are acceptable)
- [ ] `argo vertical show procurement-assistant --json` returns valid envelope (id / name / version / persona / memory_entries / eval_seed_count ≥ 8 / vertical_toml)
- [ ] All 4 skill files have YAML frontmatter (`name` + `description` + `inputs` + `outputs`)
- [ ] `eval/seed.jsonl` has ≥ 8 fixtures, ≥ 3 of which are escalation cases (over-budget / unapproved-vendor / sanctions-hit)
- [ ] `auto_approval.toml` wake list contains only draft_rfq + compare_bids + vendor_health_check + contract_review; place / sign / approve / renew gated through `[escalation].human_approval_required`
- [ ] System prompt names budget-ceiling and 3-quote-rule explicitly
- [ ] No Rust changes; no migrations
- [ ] Every new file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vertical-procurement-assistant`
- PR title: `vertical: procurement-assistant (mission-vertical-procurement-assistant)`

## Notes for the implementer

- Treat the just-merged `verticals/executive-assistant/` and `verticals/research-analyst/` as 1:1 templates — same file shape, same YAML frontmatter, same `[escalation]` block style.
- Budget-ceiling is the load-bearing constraint — `compare_bids` and `draft_rfq` must always cite the budget line + remaining balance in the system-prompt-enforced format. Eval fixtures should check this format.
- 3-quote rule for spend ≥ $5k is the standard procurement guard — bake it into the system prompt as a hard rule. Eval fixtures should include one happy path with 3 quotes and one escalation where only 1 quote was provided.
- Sanctions-watchlist is INTENTIONALLY a static memory inventory in v1 — do NOT ship live OFAC ingestion. The sanctions-hit eval fixture should use a synthetic vendor name that matches the inventory watchlist.
- TCO over sticker-price: `compare_bids` must surface 3-year TCO, not just unit price. Persona prompt names this trade-off.
- Two escalation fixtures should exercise the budget gate specifically: one where remaining-balance is insufficient (refuse + escalate) and one where the recommendation is over a per-PO ceiling (refuse + escalate).
