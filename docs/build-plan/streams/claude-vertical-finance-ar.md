# V2.2 — Finance AR demo vertical

**Owner**: claude. **Depends on**: G-genesis (#790),
demo-marketplace-seller-vertical (#818),
codex-connector-stripe (#861) merged. Fourth demo vertical
(see also: claude-demo-multi-vertical for sales/marketing/
support).

## Scope

Accounts Receivable demo vertical — proves the platform claim
"any role" with one more concrete data point. The AR worker:
- Reviews open invoices daily
- Sends polite reminder emails (gmail) at 7 / 14 / 30 days
  past due
- Escalates to human at 45 days
- Logs payment receipt notifications via Stripe webhook
  events
- Produces weekly AR aging report (uses
  claude-document-creation-executor)

This is a high-value demo because (a) every business has AR,
(b) it touches multiple shipped capabilities (gmail, stripe,
documents), (c) ROI is concrete and measurable.

## What lands

- `crates/argo-vertical-finance-ar/` (NEW crate)
- `verticals/finance-ar/` directory under repo root with:
  - `vertical.toml` — manifest
  - `persona.toml` — voice / tone (formal but warm)
  - `system_prompt.md`
  - `memory_schema.toml` — invoice tracking + payment history
  - `eval/seeds.jsonl` — 20 test scenarios (overdue / paid /
    partial / disputed)
  - `inventory/sample_invoices.json` — 10 fake invoices for demo
  - `workspace/auto_approval.toml` — emails ≤$0.10/each + day
    cap $5
- `bin/demo_finance_ar.sh` — end-to-end demo run script
- E2E smoke test in `crates/argo-e2e/tests/vertical_smoke.rs`
- Doc page at `docs/verticals/finance-ar.md`

## Acceptance criteria
- [ ] Vertical bootstraps via Genesis from charter
- [ ] Demo script runs end-to-end (mocked external calls OK)
- [ ] All 4 duties exercise (review / remind / escalate /
      receipt)
- [ ] Weekly AR aging report renders to PDF
- [ ] E2E smoke passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `claude-vertical-finance-ar`
- PR title: `verticals: Finance AR demo vertical (claude-vertical-finance-ar)`
