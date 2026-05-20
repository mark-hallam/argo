# web-workflow-template-gallery — 50+ pre-built workflow templates on /workflows/templates

**Owner**: claude. **Sizing**: ~500-800 LOC + content seed.
**Authored against**: origin/main `fe35cb61` on `2026-05-20`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-workflow-template-gallery`.
**Priority**: P1 — closes the n8n template-gallery moat. n8n has 1,400+ community templates. SKYCOT needs at least 50 to be credible.

## Scope

1. New route `/workflows/templates` — browsable template gallery
2. Group templates by vertical (Sales / Support / Finance / HR / Marketing / Ops / Eng / Compliance)
3. Each template card: name + description + step count + vertical + "Use template" CTA
4. "Use template" → opens the builder pre-populated with the template steps
5. Search + filter by vertical / risk-tier (autonomous vs approval-required)

## Initial template content (50 to seed)

Per vertical, 6-7 templates:
- **Sales**: qualify_lead, draft_outreach, book_demo, handle_inbound_form, dunning_followup, deal_alert_summary
- **Support**: triage_ticket, refund_eligibility, churn_signal_alert, kb_freshness_check, escalate_vip, weekly_report
- **Finance**: extract_invoice, vendor_reconcile, expense_audit, cash_runway_alert, board_metrics, ap_aging_summary
- **People Ops**: screen_candidate, exit_interview_synth, onboarding_check, performance_review_prep, comp_band_audit
- **Marketing**: draft_status_report, social_calendar, brand_voice_qa, lead_enrichment, campaign_perf_digest
- **Ops / Eng / Compliance**: rest as v1.x

## Acceptance criteria

- [ ] /workflows/templates renders 50+ template cards
- [ ] Search + vertical-filter work
- [ ] "Use template" creates a draft + opens the builder with it
- [ ] Each template includes a sensible step set + plain-English description
- [ ] Cards link to a per-template detail page showing the full step list before commit
