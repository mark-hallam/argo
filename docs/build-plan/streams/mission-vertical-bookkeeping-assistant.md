# Mission load-bearing wire — `bookkeeping-assistant` vertical

**Owner**: claude. **Depends on**: existing vertical pattern (verified at `verticals/marketplace-seller-browser/`); `argo vertical list/lint/show` SHIPPED.
**Authored against**: origin/main `805deb35` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-bookkeeping-assistant`.

> SOURCE: MISSION_BACKLOG.md Section 11 #147 — "Bookkeeping-assistant vertical." Adds a new vertical for transaction categorization + monthly reconciliation + report generation. Wires `[mcp_servers] quickbooks_xero` once those connectors land (currently optional — vertical works in dry-run with mock backend).

## Scope

NEW `verticals/bookkeeping-assistant/` (content-only, no Rust code) mirroring the structure at `verticals/marketplace-seller-browser/`:

1. `vertical.toml`:
   - `[vertical].id = "bookkeeping-assistant"`, version `0.1.0`
   - `[role]` persona = experienced US-based bookkeeper (focus: small-business GAAP-cash basis)
   - `[tools].allowed` = `["file_read", "web_search", "web_extract"]`
   - `[tools].denied` = `["terminal", "file_patch", "delegate_task"]`
   - `[mcp_servers]` = `{ quickbooks = { transport = "stdio", command = "argo-mcp-connector-quickbooks", args = [] } }` (optional — works in dry-run if connector missing)
   - `[escalation].human_approval_required` = `["post_journal_entry", "categorize_large_txn"]` (any transaction > $1000)
   - `[escalation].escalate_after_failures = 2`

2. `prompts/system.md` (≤ 200 LOC) — system prompt covering: GAAP-cash basis conventions, chart of accounts mapping rules, recurring-vs-one-time categorization, when to escalate (anything ambiguous, multi-line transactions).

3. `prompts/persona.md` (≤ 100 LOC) — first-person persona description.

4. `skills/categorize_transaction.md` — skill: read raw txn description, output category + suggested journal entry.

5. `skills/monthly_close.md` — skill: produce P&L + balance sheet trial, flag exceptions.

6. `skills/expense_report_summary.md` — skill: bucketize a CSV of expenses for an employee report.

7. `memory/schema.toml` — namespaces: `clients`, `chart_of_accounts`, `recurring_rules`, `flagged_transactions`.

8. `eval/seed.jsonl` (≥ 7 fixtures per mission #140): 3 clear-categorize, 2 ambiguous-needs-escalation, 2 complex-multi-line.

9. `auto_approval.toml` — approves: `categorize_transaction` (single txn under $1000), `monthly_close` (read-only). Requires-approval: `post_journal_entry`, `categorize_large_txn`.

10. `inventory.json` (small mock chart of accounts for v1 demo).

This is NOT in scope: shipping a real QuickBooks connector (separate brief `mission-mcp-connector-quickbooks-xero` exists in backlog); HIPAA / SOC2 compliance overlay; multi-currency support; tax-jurisdiction-specific rules.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `verticals/bookkeeping-assistant/vertical.toml` (NEW, ≤ 80 LOC)
- `verticals/bookkeeping-assistant/prompts/system.md` (NEW, ≤ 200 LOC)
- `verticals/bookkeeping-assistant/prompts/persona.md` (NEW, ≤ 100 LOC)
- `verticals/bookkeeping-assistant/skills/categorize_transaction.md` (NEW, ≤ 80 LOC)
- `verticals/bookkeeping-assistant/skills/monthly_close.md` (NEW, ≤ 80 LOC)
- `verticals/bookkeeping-assistant/skills/expense_report_summary.md` (NEW, ≤ 80 LOC)
- `verticals/bookkeeping-assistant/memory/schema.toml` (NEW, ≤ 40 LOC)
- `verticals/bookkeeping-assistant/eval/seed.jsonl` (NEW, ≥ 7 fixtures, ≤ 150 LOC)
- `verticals/bookkeeping-assistant/auto_approval.toml` (NEW, ≤ 40 LOC)
- `verticals/bookkeeping-assistant/inventory.json` (NEW, ≤ 80 LOC)

## Files you must NOT touch

- Any other `verticals/<name>/` directory
- Any `crates/**` source
- `crates/argo-store/migrations/**`
- `docs/build-plan/MISSION_BACKLOG.md` row content (tag flip only)

## Acceptance criteria

- [ ] `argo vertical list` includes `bookkeeping-assistant`
- [ ] `argo vertical lint bookkeeping-assistant` reports no errors
- [ ] `argo vertical show bookkeeping-assistant --json` returns valid envelope
- [ ] All 3 skill files have correct markdown frontmatter
- [ ] eval/seed.jsonl has ≥ 7 fixtures, each with `input` + `expected` keys
- [ ] auto_approval.toml gates the right actions per the brief
- [ ] No code changes
- [ ] Every new file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vertical-bookkeeping-assistant`
- PR title: `vertical: bookkeeping-assistant (mission-vertical-bookkeeping-assistant)`

## Notes for the implementer

- Use `verticals/marketplace-seller-browser/` as the structural template; copy the file layout exactly.
- The persona should feel like an actual experienced bookkeeper — specific examples, not generic phrasing.
- For eval/seed.jsonl: mix realistic small-business txn descriptions ("ACH WF Bank #1234 SAFEWAY 8.45", "Vendor: Comcast Business — $89.99/mo", etc.).
- The QuickBooks/Xero connector is intentionally optional in v1 — vertical works in dry-run with the mock inventory.json.
