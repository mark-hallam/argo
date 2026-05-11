# Mission load-bearing wire — `customer-onboarding` vertical

**Owner**: claude. **Depends on**: existing vertical pattern (verified at `verticals/marketplace-seller-browser/`).
**Authored against**: origin/main `d10658d5` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-customer-onboarding`.

> SOURCE: MISSION_BACKLOG.md Section 11 #155 — "Customer-onboarding vertical." Post-sale CSM-style work: welcome sequence + account setup verification + first-value tracking + escalation on stuck signups. Drives a templated 14-day onboarding playbook.

## Scope

NEW `verticals/customer-onboarding/` (content-only) mirroring `verticals/marketplace-seller-browser/`:

1. `vertical.toml`:
   - `[vertical].id = "customer-onboarding"`, version `0.1.0`
   - `[role]` persona = warm CSM who reads engagement signals + escalates fast
   - `[tools].allowed` = `["web_search", "web_extract", "file_read"]`
   - `[mcp_servers]` = `{ slack = {...}, hubspot = {...} }` (both optional)
   - `[escalation].human_approval_required` = `["send_email_external", "mark_churn_risk", "trigger_executive_intervention"]`
   - `[escalation].escalate_after_failures = 2`

2. `prompts/system.md` (≤ 200 LOC) — covers: 14-day playbook outline (day 0 welcome → day 3 setup-check → day 7 value-check → day 14 review), engagement signals to track (logins, feature usage, support tickets), escalation criteria (no login by day 5, churn-keyword in any reply, ticket-volume spike).

3. `prompts/persona.md` (≤ 100 LOC).

4. `skills/draft_welcome_email.md` — given account profile, produce personalized welcome with 3 setup pointers.

5. `skills/setup_health_check.md` — given account, score completeness against checklist + suggest next action.

6. `skills/draft_value_check_in.md` — given usage data, produce day-7 check-in highlighting realized value + open questions.

7. `skills/identify_churn_signals.md` — given engagement data, score churn risk + list top 3 indicators.

8. `memory/schema.toml` — namespaces: `accounts_onboarding`, `playbook_state`, `engagement_thresholds`, `intervention_outcomes`.

9. `eval/seed.jsonl` (≥ 7 fixtures): 3 healthy-onboarding, 2 at-risk, 2 escalation-needed.

10. `auto_approval.toml` — internal drafts auto. External sends + churn-mark + executive-trigger gated.

11. `inventory.json` — sample engagement thresholds + playbook template for v1 demo.

This is NOT in scope: in-product UI (separate frontend brief); demo-scheduling (that's executive-assistant or sales-sdr territory); contract renewal (post-onboarding vertical); shared-team CSM (single-CSM-per-account v1).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `verticals/customer-onboarding/vertical.toml` (NEW)
- `verticals/customer-onboarding/prompts/system.md` (NEW, ≤ 200 LOC)
- `verticals/customer-onboarding/prompts/persona.md` (NEW, ≤ 100 LOC)
- `verticals/customer-onboarding/skills/*.md` (4 NEW skill files, ≤ 80 LOC each)
- `verticals/customer-onboarding/memory/schema.toml` (NEW, ≤ 40 LOC)
- `verticals/customer-onboarding/eval/seed.jsonl` (NEW, ≥ 7 fixtures)
- `verticals/customer-onboarding/auto_approval.toml` (NEW, ≤ 40 LOC)
- `verticals/customer-onboarding/inventory.json` (NEW, ≤ 80 LOC)

## Files you must NOT touch

- Any other `verticals/<name>/` directory
- Any `crates/**` source
- `crates/argo-store/migrations/**`

## Acceptance criteria

- [ ] `argo vertical list` includes `customer-onboarding`
- [ ] `argo vertical lint customer-onboarding` reports no errors
- [ ] `argo vertical show customer-onboarding --json` returns valid envelope
- [ ] All 4 skill files have correct frontmatter
- [ ] eval/seed.jsonl has ≥ 7 fixtures
- [ ] auto_approval.toml gates external-comms + churn-mark behind human
- [ ] No code changes
- [ ] Every new file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vertical-customer-onboarding`
- PR title: `vertical: customer-onboarding (mission-vertical-customer-onboarding)`

## Notes for the implementer

- The 14-day playbook in system.md should match SaaS industry norms (e.g. Mixpanel/Pendo benchmarks for activation timing).
- For eval/seed.jsonl: use anonymized customer types ("a 50-person fintech with low logins"), not real names.
- The churn-signal thresholds are starting points — operators tune via inventory.json.
- Don't try to wire actual product-telemetry sources — engagement_thresholds + playbook_state are app-state placeholders that downstream tools populate.
