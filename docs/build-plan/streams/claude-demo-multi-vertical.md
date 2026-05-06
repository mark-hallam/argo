# V2.2 — Three more demo verticals

**Owner**: claude. **Depends on**: G-genesis,
F-skills-self-learning merged.

## Scope

v1 ships ONE demo vertical (marketplace-seller). The
mission claim "any role" needs more data points. Ship 3
more verticals proving the platform isn't single-purpose:

1. **customer-support** — answer tickets, escalate when
   stuck, log to ticket system
2. **sales-sdr** — outbound prospecting, lead qualification,
   meeting booking
3. **marketing-intern** — competitive scans, social posts
   drafted (NEVER auto-published; approval-gated), weekly
   summary

Each vertical follows the existing pattern:
- Charter (markdown role description)
- Skill bundle (3-7 skills bootstrapped)
- Genesis archetype mapping
- Demo run script (`bin/demo_<vertical>.sh`)
- Smoke test under e2e

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-vertical-customer-support/` (NEW crate,
  ~600 LOC cap)
- `crates/argo-vertical-sales-sdr/` (NEW crate, ~600 LOC)
- `crates/argo-vertical-marketing-intern/` (NEW crate,
  ~600 LOC)
- `Cargo.toml` workspace — narrowly: add 3 members
- `bin/demo_customer_support.sh` (NEW)
- `bin/demo_sales_sdr.sh` (NEW)
- `bin/demo_marketing_intern.sh` (NEW)
- `crates/argo-e2e/tests/vertical_smoke.rs` — narrowly:
  add 3 smoke cases
- `docs/verticals/customer-support.md` (NEW)
- `docs/verticals/sales-sdr.md` (NEW)
- `docs/verticals/marketing-intern.md` (NEW)

## Acceptance criteria
- [ ] 3 vertical crates compile
- [ ] Each vertical bootstraps via Genesis
- [ ] Each vertical runs its demo script end-to-end (mocked
      external calls OK)
- [ ] All 3 e2e smoke tests pass
- [ ] Marketing-intern vertical's "post" action ALWAYS
      approval-gated (no autonomous posting)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `claude-demo-multi-vertical`
- PR title: `verticals: customer-support + sales-sdr + marketing-intern demos (claude-demo-multi-vertical)`
