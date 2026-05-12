# Mission load-bearing wire — Finance-AR vertical: 5+ skills

**Owner**: claude. **Depends on**: existing `verticals/finance-ar/` charter stub.
**Authored against**: origin/main `d866e01c` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-finance-ar-skills`.

> SOURCE: MISSION_BACKLOG.md Section 4 #137, scoped to finance-ar. Today the vertical ships with 1 skill (`draft_reminder.md`). MISSION claim 7 expects parity-with-gtm-engineer. This brief brings finance-ar to 6 skills.

## Scope

5 NEW skill files under `verticals/finance-ar/skills/`. Skills cover the dominant AR flow not already covered by `draft_reminder`:

1. **`aging_report.skill.md`** — weekly aging report: bucket open invoices by 0-30 / 31-60 / 61-90 / 90+ days, totals + count per bucket, list of top-10 oldest, recommended next-step per bucket (reminder / escalation / write-off review). Output Markdown report + summary JSON. Approval-gated before publishing.
2. **`escalate_past_due.skill.md`** — at 45+ days past due (charter threshold), package the account for a human handoff: invoice numbers + amounts + reminder history + last contact + recommended action (collections call / payment plan offer / write-off). Output: escalation envelope.
3. **`log_payment_receipt.skill.md`** — when a payment lands (operator pastes confirmation), match it to an open invoice, log to `memory.payment_history[]`, mark the invoice paid in the report cache. Handles partial payments (records remainder), wrong-customer payments (flag for human review), overpayments (flag for refund).
4. **`reconcile_invoice.skill.md`** — given an apparent discrepancy (customer says "I already paid"), pull invoice + payment history + emit a reconciliation summary: what we billed / what we received / gap / recommended next step (apply payment / dispute / waive). Approval-gated before sending the customer-facing reply.
5. **`payment_plan_proposal.skill.md`** — given an at-risk account, propose a 3-tier payment plan: minimal (15-30-45 days), balanced (10-20-30), aggressive (5-15-25). Each tier has the math (per-installment amount, interest if applicable, default consequence). Output JSON; downstream approval before offering.

Each skill ~50-80 LOC. Total: ~300-400 LOC.

This is NOT in scope: changing `vertical.toml`; modifying prompts; expanding `eval/seed.jsonl`; live Stripe/QuickBooks integration (Tier A connectors are separate); auto-sending customer-facing emails.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `verticals/finance-ar/skills/aging_report.skill.md` (NEW, ≤ 90 LOC)
- `verticals/finance-ar/skills/escalate_past_due.skill.md` (NEW, ≤ 80 LOC)
- `verticals/finance-ar/skills/log_payment_receipt.skill.md` (NEW, ≤ 80 LOC)
- `verticals/finance-ar/skills/reconcile_invoice.skill.md` (NEW, ≤ 80 LOC)
- `verticals/finance-ar/skills/payment_plan_proposal.skill.md` (NEW, ≤ 80 LOC)

## Files you must NOT touch

- `verticals/finance-ar/skills/draft_reminder.md` — existing skill stable
- `verticals/finance-ar/vertical.toml` — schema is fine, skills auto-discovered
- `verticals/finance-ar/prompts/**` — persona stable
- Any other vertical's skill directory

## Acceptance criteria

- [ ] 5 new `.skill.md` files
- [ ] Each has frontmatter: `name`, `version: "0.1.0"`, `description`, `provenance: { source: vertical/finance-ar, hash: sha256-pending }`
- [ ] Each has sections: `# When to use`, `# Inputs you expect`, `# Output shape`, `# Examples`
- [ ] `aging_report` produces deterministic bucket totals (0-30/31-60/61-90/90+) — operator can spot-check arithmetic
- [ ] `escalate_past_due` triggers at the charter's 45-day threshold (don't drift)
- [ ] `log_payment_receipt` handles 3 edge cases: partial / wrong-customer / overpayment, each with a distinct output flag
- [ ] `reconcile_invoice` outputs are marked `requires_approval: true`
- [ ] `payment_plan_proposal` shows the math (per-installment amount + total) so the operator can verify before approving
- [ ] No file > 100 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vertical-finance-ar-skills`
- PR title: `verticals: 5 skills for finance-ar (mission-vertical-finance-ar-skills)`

## Notes for the implementer

- Model after `verticals/gtm-engineer/skills/research_company.skill.md` for shape. Tone should be precise + auditable — finance ops doesn't want flowery language.
- The 45-day escalation threshold lives in the charter description. If you find a different threshold elsewhere in the vertical, surface it as a question — don't paper over the drift.
- For `reconcile_invoice`, the four outcomes (apply payment / dispute / waive / no-action) form a closed set. Document them explicitly in the skill's `# Output shape` so the runner doesn't synthesize a fifth.
- `payment_plan_proposal` math: show formulas, not just final numbers. Example: "tier 1 = total / 3, due [today + 30d, +60d, +90d]". The skill is the source of truth; the operator needs to be able to audit the arithmetic.
- Synthetic data only — finance-ar's charter doesn't have a "PHI / PII" gate but it does carry real-money side effects. Skills should never auto-execute a Stripe write; every customer-facing artifact is approval-gated.
