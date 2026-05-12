# Mission load-bearing wire — Bookkeeping-Assistant vertical: 3+ skills to reach 6

**Owner**: claude. **Depends on**: existing `verticals/bookkeeping-assistant/` charter stub.
**Authored against**: origin/main `df7b9dfe` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-bookkeeping-assistant-skills`.

> SOURCE: MISSION_BACKLOG.md Section 4 #137, scoped to bookkeeping-assistant. Today the vertical ships with 3 skills (categorize_transaction / expense_report_summary / monthly_close). MISSION claim 7 expects parity-with-gtm-engineer (8 skills). This brief brings bookkeeping-assistant to 6 — closing #137 for this vertical.

## Scope

3 NEW skill files under `verticals/bookkeeping-assistant/skills/`:

1. **`reconcile_bank_statement.skill.md`** — given a bank statement (CSV / OFX / operator-pasted list) and the current ledger, output reconciliation envelope: matched transactions, unmatched bank entries, unmatched ledger entries, suggested categorization for each unmatched bank entry. Approval-gated before posting unmatched entries.
2. **`flag_anomaly.skill.md`** — given recent transactions + historical pattern, surface anomalies: duplicate entries, sudden category-spend spikes (>2σ from 90-day median), missing recurring transactions (rent / payroll), round-number suspicious entries (potential fraud indicator). Output JSON with severity (info / advisory / urgent) + suggested investigation steps.
3. **`generate_pl_summary.skill.md`** — given a date range + the categorized ledger, produce a P&L summary: revenue / COGS / opex / EBITDA / margin% with month-over-month + year-over-year deltas (when data available). Output Markdown report + JSON metadata. Internal-output; not approval-gated.

Each skill ~50-90 LOC. Total: ~200-270 LOC.

This is NOT in scope: live QuickBooks/Xero write (charter says "Optional QuickBooks/Xero MCP connector; works in dry-run"); tax-form generation (separate vertical: tax-prep); auto-posting unmatched entries; multi-currency reconciliation.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `verticals/bookkeeping-assistant/skills/reconcile_bank_statement.skill.md` (NEW, ≤ 90 LOC)
- `verticals/bookkeeping-assistant/skills/flag_anomaly.skill.md` (NEW, ≤ 90 LOC)
- `verticals/bookkeeping-assistant/skills/generate_pl_summary.skill.md` (NEW, ≤ 90 LOC)

## Files you must NOT touch

- Existing skill files `categorize_transaction.md` / `expense_report_summary.md` / `monthly_close.md`
- `verticals/bookkeeping-assistant/vertical.toml`, prompts — locked
- Any other vertical's skill directory

## Acceptance criteria

- [ ] 3 new `.skill.md` files
- [ ] Each has frontmatter: `name`, `version: "0.1.0"`, `description`, `provenance: { source: vertical/bookkeeping-assistant, hash: sha256-pending }`
- [ ] Each has sections: `# When to use`, `# Inputs you expect`, `# Output shape`, `# Examples`, `# Approval posture`
- [ ] `reconcile_bank_statement` is approval-gated for any posting of new entries
- [ ] `flag_anomaly` documents the 3-severity vocabulary (info / advisory / urgent) + the 2σ threshold for category spikes
- [ ] `generate_pl_summary` deterministic arithmetic — operator can spot-check totals against ledger
- [ ] No skill auto-posts to QuickBooks/Xero
- [ ] No file > 100 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vertical-bookkeeping-assistant-skills`
- PR title: `verticals: 3 skills for bookkeeping-assistant (mission-vertical-bookkeeping-assistant-skills)`

## Notes for the implementer

- US GAAP-cash-basis is the v1 scope per the charter. Accrual accounting (`unearned_revenue`, `prepaid_expense`) is out of scope; the skills should refuse to enter accrual-only categories and route to the operator for advice.
- `flag_anomaly`'s round-number heuristic is an honest fraud-detection seam: small-business books sometimes show suspicious round-number entries that hide skim. Document the threshold (e.g. ≥3 round-100 entries in a week) and the auditor-readable framing.
- `reconcile_bank_statement` is the highest-value skill for the persona. The output should pair each ledger entry with a confidence score — high-confidence matches can be auto-approved by policy; low-confidence ones surface to the operator.
- For `generate_pl_summary`, prefer side-by-side period comparison over a single-period table — operators care about trends, not point-in-time.
