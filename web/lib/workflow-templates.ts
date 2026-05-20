/**
 * Workflow template seed — 50+ pre-built playbooks for `/workflows/templates`.
 *
 * Closes the n8n template-gallery moat (n8n has 1,400+ community templates).
 * V1 ships 50 curated templates across eight verticals so the gallery has
 * body and the operator demo lands credibly. Each template is a step list
 * the builder can pre-populate when the operator clicks "Use template".
 *
 * The `WorkflowTemplate` / `TemplateStep` shape intentionally mirrors the
 * `WorkflowStep` shape in `app/(app)/workflows/[verticalId]/[name]/build/page.tsx`
 * so the builder can swap a template in with no transformation.
 *
 * Real binding to gateway-stored templates lands in a follow-up brief; this
 * file is the v1 seed.
 */
export type TemplateStepKind =
  | "trigger"
  | "skill_invoke"
  | "approval"
  | "tool_call"
  | "notify"
  | "decision"
  | "data";

export interface TemplateStep {
  id: string;
  kind: TemplateStepKind;
  name: string;
  description: string;
  approvalGate?: { reason: string; channel: string };
  inputs?: string[];
  outputs?: string[];
}

export type TemplateVertical =
  | "sales"
  | "support"
  | "finance"
  | "people-ops"
  | "marketing"
  | "ops"
  | "engineering"
  | "compliance";

export type TemplateRisk = "approval-required" | "autonomous";

export interface WorkflowTemplate {
  /** kebab-case stable identifier, unique across the seed. */
  id: string;
  /** snake_case workflow name as it would appear in the builder. */
  name: string;
  vertical: TemplateVertical;
  /** plain-English headline shown on the gallery card. */
  description: string;
  risk: TemplateRisk;
  /** ordered step list — drives the builder canvas pre-populate. */
  steps: TemplateStep[];
}

export const VERTICAL_LABEL: Record<TemplateVertical, string> = {
  sales: "Sales",
  support: "Support",
  finance: "Finance",
  "people-ops": "People Ops",
  marketing: "Marketing",
  ops: "Operations",
  engineering: "Engineering",
  compliance: "Compliance",
};

export const RISK_LABEL: Record<TemplateRisk, string> = {
  "approval-required": "Approval required",
  autonomous: "Autonomous",
};

export const RISK_COLOR: Record<TemplateRisk, string> = {
  "approval-required": "#f59e0b",
  autonomous: "#14b8a6",
};

const t = (
  id: string,
  name: string,
  vertical: TemplateVertical,
  description: string,
  risk: TemplateRisk,
  steps: TemplateStep[],
): WorkflowTemplate => ({ id, name, vertical, description, risk, steps });

const TRIGGER_INBOUND: TemplateStep = {
  id: "s1",
  kind: "trigger",
  name: "Inbound event arrives",
  description: "Webhook / inbox / queue message fires the workflow.",
};

export const WORKFLOW_TEMPLATES: WorkflowTemplate[] = [
  // ── Sales (8) ──────────────────────────────────────────────────────────
  t("sales-qualify-lead", "qualify_lead", "sales",
    "BANT-style qualification + calendar-link insertion + CRM update on accept.",
    "approval-required",
    [
      { id: "s1", kind: "trigger", name: "Lead arrives", description: "HubSpot/Salesforce webhook for new form-fill, MQL, or referral." },
      { id: "s2", kind: "skill_invoke", name: "Enrich + BANT score", description: "Firmographic + technographic data; 0-100 BANT score.", outputs: ["bant_score", "enrichment"] },
      { id: "s3", kind: "decision", name: "Branch on BANT", description: "≥75 = demo invite. 50-74 = nurture. <50 = polite decline." },
      { id: "s4", kind: "approval", name: "AE approves outreach", description: "Slack DM with qual summary + one-tap approve.", approvalGate: { reason: "Outreach to enterprise needs AE signoff.", channel: "Slack" } },
      { id: "s5", kind: "tool_call", name: "Send outreach + calendar link", description: "Gmail/Outlook send + Calendly slot + CRM update." },
    ],
  ),
  t("sales-draft-outreach", "draft_outreach", "sales",
    "Personalized cold-outreach email + LinkedIn message drafted in your tone.",
    "approval-required",
    [
      { id: "s1", kind: "trigger", name: "Prospect added to list", description: "Sales loaded an ICP list; one item at a time enters." },
      { id: "s2", kind: "skill_invoke", name: "Research prospect", description: "Recent funding, role change, public posts; 3 personalization hooks.", outputs: ["hooks[]"] },
      { id: "s3", kind: "skill_invoke", name: "Draft email + LI message", description: "Tone-matched, 90-word email + 2-sentence LI opener." },
      { id: "s4", kind: "approval", name: "AE approves draft", description: "Inline preview + one-tap edit/approve.", approvalGate: { reason: "Cold outreach must not be sent without rep approval.", channel: "Slack" } },
      { id: "s5", kind: "tool_call", name: "Send + log", description: "Gmail send + CRM activity log + sequence enroll." },
    ],
  ),
  t("sales-book-demo", "book_demo", "sales",
    "Inbound demo request → SDR-quality screen + calendar slot offer + reminder cadence.",
    "approval-required",
    [
      { id: "s1", kind: "trigger", name: "Demo form submitted", description: "Web form / pricing-page CTA fires." },
      { id: "s2", kind: "skill_invoke", name: "Verify ICP fit", description: "Domain + headcount + use-case classify; flag obvious mismatches." },
      { id: "s3", kind: "decision", name: "ICP fit?", description: "Yes → proceed. No → polite redirect to self-serve docs." },
      { id: "s4", kind: "tool_call", name: "Offer calendar slots", description: "Send Calendly link tied to right AE pod." },
      { id: "s5", kind: "approval", name: "AE confirms booking", description: "AE one-tap accept / reschedule.", approvalGate: { reason: "AE owns the discovery call.", channel: "Slack" } },
      { id: "s6", kind: "notify", name: "Reminder cadence", description: "Day-before + day-of email + Slack reminder to AE." },
    ],
  ),
  t("sales-handle-inbound-form", "handle_inbound_form", "sales",
    "Triage inbound contact-us forms — route to sales, support, or partnerships.",
    "autonomous",
    [
      TRIGGER_INBOUND,
      { id: "s2", kind: "skill_invoke", name: "Classify intent", description: "Sales / support / partner / press / spam. Confidence 0-1." },
      { id: "s3", kind: "decision", name: "Branch on intent", description: "Each intent routes to its owning queue." },
      { id: "s4", kind: "tool_call", name: "Route to queue", description: "CRM lead create OR ticket open OR partner-CRM add." },
      { id: "s5", kind: "notify", name: "Acknowledge", description: "Auto-reply email confirming the right team will follow up." },
    ],
  ),
  t("sales-dunning-followup", "dunning_followup", "sales",
    "Payment-failed dunning sequence with empathy + retry windows + escalation.",
    "approval-required",
    [
      { id: "s1", kind: "trigger", name: "Invoice marked failed", description: "Stripe / Chargebee webhook on payment failure." },
      { id: "s2", kind: "skill_invoke", name: "Compose dunning email", description: "Tone-aware, mentions specific invoice + retry option." },
      { id: "s3", kind: "tool_call", name: "Send + retry", description: "Email + scheduled retry charge in 72h." },
      { id: "s4", kind: "decision", name: "Still failing after 7d?", description: "Yes → escalate to CSM. No → close." },
      { id: "s5", kind: "approval", name: "CSM approves escalation copy", description: "Final-notice email needs human signoff.", approvalGate: { reason: "Final-notice tone matters for retention.", channel: "Slack" } },
    ],
  ),
  t("sales-deal-alert-summary", "deal_alert_summary", "sales",
    "Daily Slack digest of deal-room movement — stage shifts, stalls, fresh signals.",
    "autonomous",
    [
      { id: "s1", kind: "trigger", name: "Daily 7am cron", description: "Fires every business day at 07:00 sales-team timezone." },
      { id: "s2", kind: "data", name: "Fetch deal-room changes", description: "Diff Salesforce vs yesterday's snapshot.", outputs: ["stage_shifts", "new_signals", "stalls"] },
      { id: "s3", kind: "skill_invoke", name: "Summarize", description: "5-bullet digest grouped by AE + deal value." },
      { id: "s4", kind: "notify", name: "Post to #revenue", description: "Slack post with one-line per deal + link to record." },
    ],
  ),
  t("sales-competitor-intel", "competitor_intel_digest", "sales",
    "Weekly competitor-watch report — pricing changes, new features, hiring signals.",
    "autonomous",
    [
      { id: "s1", kind: "trigger", name: "Monday 6am cron", description: "Weekly Monday digest." },
      { id: "s2", kind: "tool_call", name: "Crawl competitor pages", description: "Pricing pages + blog feeds + LinkedIn job posts." },
      { id: "s3", kind: "skill_invoke", name: "Synthesize signals", description: "What changed vs last week, ranked by sales relevance." },
      { id: "s4", kind: "notify", name: "Email + post to #sales", description: "Markdown digest emailed to leadership + Slack post." },
    ],
  ),
  t("sales-renewal-prep", "renewal_prep_brief", "sales",
    "30 days before renewal — usage delta + key-stakeholder map + risk score.",
    "approval-required",
    [
      { id: "s1", kind: "trigger", name: "30d-before-renewal cron", description: "Fires when a contract is exactly 30 days from expiry." },
      { id: "s2", kind: "data", name: "Pull usage + adoption", description: "DAU/WAU, feature mix, support tickets, NPS." },
      { id: "s3", kind: "skill_invoke", name: "Score risk + draft brief", description: "Red/amber/green + 1-page CSM brief." },
      { id: "s4", kind: "approval", name: "CSM signs off", description: "CSM tweaks talking points before save.", approvalGate: { reason: "CSM owns the renewal narrative.", channel: "Slack" } },
      { id: "s5", kind: "notify", name: "Drop brief in account folder", description: "Saved to the account's Google Drive folder + Slack ping." },
    ],
  ),

  // ── Support (8) ────────────────────────────────────────────────────────
  t("support-triage-ticket", "triage_ticket", "support",
    "Classify tickets, deflect with KB articles, escalate the rest.",
    "autonomous",
    [
      { id: "s1", kind: "trigger", name: "New support ticket", description: "Zendesk / Intercom / Front webhook on ticket-create." },
      { id: "s2", kind: "skill_invoke", name: "Classify + tag", description: "Category + priority + customer-tier; sentiment 0-1.", outputs: ["category", "priority", "sentiment"] },
      { id: "s3", kind: "decision", name: "Can deflect?", description: "If KB match ≥0.8 confidence → deflect, else → human queue." },
      { id: "s4", kind: "tool_call", name: "Send KB reply OR route", description: "Either KB-quote auto-reply or queue assignment + tags." },
    ],
  ),
  t("support-refund-eligibility", "refund_eligibility", "support",
    "Verify refund eligibility, check ledger, route high-value to manager.",
    "approval-required",
    [
      { id: "s1", kind: "trigger", name: "Refund requested", description: "Customer requests a refund in ticket or self-serve form." },
      { id: "s2", kind: "skill_invoke", name: "Check policy", description: "Within window? Subscription type? Prior refunds? Eligible Y/N + reason." },
      { id: "s3", kind: "decision", name: "Amount > $500?", description: "Yes → manager approval. No → CSR can decide." },
      { id: "s4", kind: "approval", name: "Manager approves high-value", description: "Slack approval with full context.", approvalGate: { reason: "High-value refunds need manager sign-off per policy.", channel: "Slack" } },
      { id: "s5", kind: "tool_call", name: "Process refund + reply", description: "Stripe refund + customer email + ticket close." },
    ],
  ),
  t("support-churn-signal-alert", "churn_signal_alert", "support",
    "Catch ticket-language patterns that historically precede churn; alert CSM.",
    "autonomous",
    [
      { id: "s1", kind: "trigger", name: "New ticket arrives", description: "Every customer-initiated ticket enters." },
      { id: "s2", kind: "skill_invoke", name: "Scan for churn cues", description: "Frustration, comparison shopping, contract terms, escalation language." },
      { id: "s3", kind: "decision", name: "Risk ≥ 0.7?", description: "Trigger alert if confidence above threshold." },
      { id: "s4", kind: "notify", name: "Ping CSM in Slack", description: "DM with ticket link + flagged phrases + recommended action." },
    ],
  ),
  t("support-kb-freshness-check", "kb_freshness_check", "support",
    "Weekly audit of KB articles — last-updated, deflection rate, contradiction with current product.",
    "approval-required",
    [
      { id: "s1", kind: "trigger", name: "Monday 8am cron", description: "Weekly KB audit." },
      { id: "s2", kind: "data", name: "Pull KB metadata + usage", description: "Last-updated, view count, deflection-success rate." },
      { id: "s3", kind: "skill_invoke", name: "Flag stale + contradictory", description: "Compare against changelog + current docs; flag outdated phrasing." },
      { id: "s4", kind: "approval", name: "Lead reviews flag list", description: "Approve the list of articles to rewrite/retire.", approvalGate: { reason: "KB changes are public-facing.", channel: "Slack" } },
      { id: "s5", kind: "notify", name: "Open rewrite tickets", description: "Linear ticket per article + Slack post to #support-content." },
    ],
  ),
  t("support-escalate-vip", "escalate_vip", "support",
    "VIP-tier customer waits >15min → instant escalation with full context.",
    "autonomous",
    [
      { id: "s1", kind: "trigger", name: "Ticket-age cron @5min", description: "Polls open tickets every 5 minutes." },
      { id: "s2", kind: "data", name: "Filter VIP + waiting >15m", description: "Match against tier list; check first-response timer." },
      { id: "s3", kind: "skill_invoke", name: "Summarize context", description: "1-paragraph summary + account value + recent ticket history." },
      { id: "s4", kind: "notify", name: "Page on-call lead", description: "PagerDuty + Slack #support-vip + email to head of support." },
    ],
  ),
  t("support-weekly-report", "weekly_report", "support",
    "Friday digest — volume, deflection, CSAT, top categories, top escalations.",
    "autonomous",
    [
      { id: "s1", kind: "trigger", name: "Friday 4pm cron", description: "Weekly support digest." },
      { id: "s2", kind: "data", name: "Pull metrics", description: "Volume, FCR, CSAT, NPS, deflection rate, top tags." },
      { id: "s3", kind: "skill_invoke", name: "Compose digest", description: "Narrative summary with WoW deltas + 3 callouts." },
      { id: "s4", kind: "notify", name: "Email leadership + post #support", description: "Markdown + chart-image attachments." },
    ],
  ),
  t("support-csat-drop-alert", "csat_drop_alert", "support",
    "Detect rolling CSAT dips below floor and surface likely causes.",
    "autonomous",
    [
      { id: "s1", kind: "trigger", name: "Hourly metric poll", description: "Pull rolling 24h CSAT every hour." },
      { id: "s2", kind: "decision", name: "Dipped below 4.2?", description: "Compare to 30-day baseline." },
      { id: "s3", kind: "skill_invoke", name: "Likely cause cluster", description: "Group recent negative comments + tag patterns." },
      { id: "s4", kind: "notify", name: "Alert head of support", description: "Slack DM + dashboard link." },
    ],
  ),
  t("support-incident-status-page", "incident_status_update", "support",
    "When incident raised, draft customer-facing status-page update + ticket macros.",
    "approval-required",
    [
      { id: "s1", kind: "trigger", name: "Incident page-up", description: "PagerDuty incident SEV-1 or SEV-2 fired." },
      { id: "s2", kind: "skill_invoke", name: "Draft status update", description: "Customer-tone, no engineering jargon, what + impact + ETA." },
      { id: "s3", kind: "approval", name: "Incident commander approves", description: "Slack one-tap approve before publish.", approvalGate: { reason: "Status page is public + lawyer-sensitive.", channel: "Slack" } },
      { id: "s4", kind: "tool_call", name: "Publish + macro update", description: "Statuspage post + Zendesk macro refresh." },
    ],
  ),

  // ── Finance (8) ────────────────────────────────────────────────────────
  t("finance-extract-invoice", "extract_invoice", "finance",
    "Pull line items from PDF/scanned invoices, reconcile vendor, queue for approval.",
    "autonomous",
    [
      { id: "s1", kind: "trigger", name: "Invoice arrives in AP inbox", description: "AP email forwards or vendor portal webhook." },
      { id: "s2", kind: "skill_invoke", name: "OCR + parse line items", description: "Extract vendor, amount, line items, due date, PO ref." },
      { id: "s3", kind: "tool_call", name: "Reconcile to PO", description: "Match against PO + 3-way-match check." },
      { id: "s4", kind: "data", name: "Stage in AP queue", description: "Write to AP system, tag for approver." },
    ],
  ),
  t("finance-vendor-reconcile", "vendor_reconcile", "finance",
    "Monthly vendor-statement reconciliation — invoices vs payments vs ledger.",
    "approval-required",
    [
      { id: "s1", kind: "trigger", name: "First-of-month cron", description: "Runs on the 1st business day each month." },
      { id: "s2", kind: "data", name: "Pull statements + ledger", description: "Vendor statements, AP register, GL postings." },
      { id: "s3", kind: "skill_invoke", name: "Find mismatches", description: "Missing payments, duplicates, off-amounts; rank by $ impact." },
      { id: "s4", kind: "approval", name: "Controller approves write-ups", description: "Review the mismatch list before AP outreach.", approvalGate: { reason: "Controller owns vendor relationships.", channel: "Slack" } },
      { id: "s5", kind: "notify", name: "Email vendors for clarification", description: "Auto-compose, controller-tone, sent from AP shared inbox." },
    ],
  ),
  t("finance-expense-audit", "expense_audit", "finance",
    "Audit submitted expenses against policy — flag duplicates, out-of-policy, missing receipts.",
    "approval-required",
    [
      { id: "s1", kind: "trigger", name: "Expense submitted", description: "Brex/Ramp/Expensify webhook on submission." },
      { id: "s2", kind: "skill_invoke", name: "Policy check", description: "Category + amount + receipt + meal/travel rules." },
      { id: "s3", kind: "decision", name: "Out-of-policy?", description: "If yes → block + ask for fix. If grey-area → human." },
      { id: "s4", kind: "approval", name: "Manager + finance review", description: "Edge cases reviewed before reimbursement.", approvalGate: { reason: "Policy edge cases need human judgment.", channel: "Slack" } },
    ],
  ),
  t("finance-cash-runway-alert", "cash_runway_alert", "finance",
    "Daily cash-runway report — burn vs reserve vs trailing-month spend.",
    "autonomous",
    [
      { id: "s1", kind: "trigger", name: "Daily 7am cron", description: "Weekday morning briefing." },
      { id: "s2", kind: "data", name: "Pull cash position", description: "Bank balances + AP + AR + recurring spend." },
      { id: "s3", kind: "skill_invoke", name: "Compute runway", description: "Months of runway at current burn + 30/60/90 burn scenarios." },
      { id: "s4", kind: "notify", name: "DM CFO + CEO", description: "Slack DM if runway <12m, else email digest." },
    ],
  ),
  t("finance-board-metrics", "board_metrics_pack", "finance",
    "Pre-board-meeting metrics pack — ARR, NRR, burn, headcount, top wins/losses.",
    "approval-required",
    [
      { id: "s1", kind: "trigger", name: "T-7d board meeting cron", description: "Fires a week before the next board meeting." },
      { id: "s2", kind: "data", name: "Pull KPI snapshot", description: "ARR, NRR, gross/net retention, burn, cash, headcount, pipeline." },
      { id: "s3", kind: "skill_invoke", name: "Compose narrative", description: "Slide-deck-ready bullets per KPI + commentary on movement." },
      { id: "s4", kind: "approval", name: "CFO + CEO approve", description: "Two-step approval before board send.", approvalGate: { reason: "Board materials are scrutinized; CEO + CFO must align.", channel: "Slack" } },
      { id: "s5", kind: "notify", name: "Send to board portal", description: "DocSend upload + email link to board." },
    ],
  ),
  t("finance-ap-aging-summary", "ap_aging_summary", "finance",
    "Weekly AP-aging report — bills due, overdue, vendor concentration.",
    "autonomous",
    [
      { id: "s1", kind: "trigger", name: "Monday 9am cron", description: "Weekly AP review." },
      { id: "s2", kind: "data", name: "AP aging buckets", description: "0-30 / 31-60 / 61-90 / 90+ + vendor concentration top 10." },
      { id: "s3", kind: "skill_invoke", name: "Flag risks", description: "Vendors at risk of service hold, payment-term mismatches." },
      { id: "s4", kind: "notify", name: "Email controller + AP team", description: "Weekly AP digest." },
    ],
  ),
  t("finance-budget-variance-alert", "budget_variance_alert", "finance",
    "Mid-month spot-check — actuals vs budget by department, flag >10% variance.",
    "autonomous",
    [
      { id: "s1", kind: "trigger", name: "15th-of-month cron", description: "Mid-month variance check." },
      { id: "s2", kind: "data", name: "Pull MTD actuals vs plan", description: "Per-department, per-category." },
      { id: "s3", kind: "skill_invoke", name: "Compute variance + explain", description: "Variance > ±10% gets a one-line driver analysis." },
      { id: "s4", kind: "notify", name: "Email dept heads", description: "Per-head digest of their own variance items only." },
    ],
  ),
  t("finance-revenue-recognition-check", "rev_rec_sanity_check", "finance",
    "Pre-close: scan deals booked this month for rev-rec edge cases.",
    "approval-required",
    [
      { id: "s1", kind: "trigger", name: "T-3d month-end cron", description: "Runs 3 business days before close." },
      { id: "s2", kind: "data", name: "Pull bookings + contract terms", description: "All deals booked this month with non-standard terms." },
      { id: "s3", kind: "skill_invoke", name: "Flag complex rev-rec", description: "Multi-element, ramp-up, prepay, services-bundle." },
      { id: "s4", kind: "approval", name: "Accounting reviews list", description: "Confirm rev-rec treatment per deal.", approvalGate: { reason: "Audit risk if mis-recognized.", channel: "Slack" } },
    ],
  ),

  // ── People Ops (7) ─────────────────────────────────────────────────────
  t("people-screen-candidate", "screen_candidate", "people-ops",
    "First-pass candidate screen against a JD with skill-match scoring.",
    "autonomous",
    [
      { id: "s1", kind: "trigger", name: "Application submitted", description: "Greenhouse / Lever webhook on new application." },
      { id: "s2", kind: "skill_invoke", name: "Score against JD", description: "Skills + experience + signal match; 0-100 score + 3 highlights." },
      { id: "s3", kind: "decision", name: "Score ≥ 70?", description: "Move to recruiter review. Else polite decline." },
      { id: "s4", kind: "tool_call", name: "Update ATS + notify recruiter", description: "Greenhouse stage + Slack ping." },
    ],
  ),
  t("people-exit-interview-synth", "exit_interview_synth", "people-ops",
    "Quarterly synthesis of exit-interview transcripts into a retention report.",
    "approval-required",
    [
      { id: "s1", kind: "trigger", name: "End-of-quarter cron", description: "Q1/Q2/Q3/Q4 close." },
      { id: "s2", kind: "data", name: "Pull exit-interview notes", description: "Privacy-scrubbed transcripts from the last quarter." },
      { id: "s3", kind: "skill_invoke", name: "Theme + cluster", description: "Top themes ranked by frequency + impact." },
      { id: "s4", kind: "approval", name: "Head of People reviews", description: "Edit + add commentary before exec send.", approvalGate: { reason: "Sensitive themes; needs human judgment.", channel: "Slack" } },
      { id: "s5", kind: "notify", name: "Send to exec team", description: "Email + Notion page update." },
    ],
  ),
  t("people-onboarding-check", "onboarding_check", "people-ops",
    "Day-30 and day-60 onboarding pulse — IT setup, training, manager check-ins.",
    "autonomous",
    [
      { id: "s1", kind: "trigger", name: "Day-30 / Day-60 cron", description: "Fires per new hire on their 30th and 60th day." },
      { id: "s2", kind: "data", name: "Check IT + training + 1:1s", description: "Confluence training completed? IT tickets resolved? 1:1 cadence kept?" },
      { id: "s3", kind: "skill_invoke", name: "Score onboarding health", description: "RAG status + specific gaps." },
      { id: "s4", kind: "notify", name: "DM manager + people-partner", description: "Slack DM with action items if amber/red." },
    ],
  ),
  t("people-performance-review-prep", "performance_review_prep", "people-ops",
    "Pre-cycle: compile each employee's wins, peer feedback, calibration data.",
    "approval-required",
    [
      { id: "s1", kind: "trigger", name: "Cycle-start cron", description: "Fires when cycle opens (H1 or H2)." },
      { id: "s2", kind: "data", name: "Pull peer feedback + wins", description: "Lattice/CultureAmp + Linear shipped + Slack kudos." },
      { id: "s3", kind: "skill_invoke", name: "Compose review brief", description: "Per-person packet for the manager." },
      { id: "s4", kind: "approval", name: "Manager reviews + edits", description: "Manager owns the final draft.", approvalGate: { reason: "Reviews are the manager's responsibility.", channel: "Slack" } },
    ],
  ),
  t("people-comp-band-audit", "comp_band_audit", "people-ops",
    "Quarterly audit of comp vs band — flag drift, gender/role outliers.",
    "approval-required",
    [
      { id: "s1", kind: "trigger", name: "Quarterly cron", description: "Q1/Q2/Q3/Q4 comp review." },
      { id: "s2", kind: "data", name: "Pull comp + band metadata", description: "Salary + equity + level + tenure + demographics." },
      { id: "s3", kind: "skill_invoke", name: "Detect drift + outliers", description: "Stat tests for systematic gaps; flag for review." },
      { id: "s4", kind: "approval", name: "Head of People + Finance", description: "Joint review before any comp action.", approvalGate: { reason: "Comp is legally sensitive.", channel: "Slack" } },
    ],
  ),
  t("people-headcount-gap-alert", "headcount_gap_alert", "people-ops",
    "Detect open-req aging + interview-loop stalls; alert head of recruiting.",
    "autonomous",
    [
      { id: "s1", kind: "trigger", name: "Daily 8am cron", description: "Daily recruiting pulse." },
      { id: "s2", kind: "data", name: "ATS open-req + stage durations", description: "Greenhouse stage-aging report." },
      { id: "s3", kind: "skill_invoke", name: "Flag stalls", description: "Reqs >30d open + loops with >7d gap between rounds." },
      { id: "s4", kind: "notify", name: "DM head of recruiting", description: "Slack with stall list + recommended unblockers." },
    ],
  ),
  t("people-offer-letter-draft", "offer_letter_draft", "people-ops",
    "When candidate moves to offer stage — draft letter, salary band check, equity calc.",
    "approval-required",
    [
      { id: "s1", kind: "trigger", name: "Stage = offer", description: "Greenhouse stage flips to offer." },
      { id: "s2", kind: "data", name: "Pull comp band + market data", description: "Internal band + benchmarking source." },
      { id: "s3", kind: "skill_invoke", name: "Draft letter + comp math", description: "Boilerplate + level-specific salary + equity grant." },
      { id: "s4", kind: "approval", name: "Recruiter + hiring manager", description: "Two-step approval before send.", approvalGate: { reason: "Offers are binding.", channel: "Slack" } },
      { id: "s5", kind: "tool_call", name: "Send via DocuSign", description: "DocuSign envelope + Slack notify candidate." },
    ],
  ),

  // ── Marketing (7) ──────────────────────────────────────────────────────
  t("marketing-draft-status-report", "draft_status_report", "marketing",
    "Weekly client status report — campaign perf + content shipped + next-week asks.",
    "approval-required",
    [
      { id: "s1", kind: "trigger", name: "Friday 3pm cron", description: "Per agency client, end-of-week digest." },
      { id: "s2", kind: "data", name: "Pull campaign metrics", description: "GA4 + ad-platform + content-CMS." },
      { id: "s3", kind: "skill_invoke", name: "Compose status doc", description: "Wins, in-flight, asks; client-friendly tone." },
      { id: "s4", kind: "approval", name: "AM reviews", description: "Account manager edits before client send.", approvalGate: { reason: "AM owns the client narrative.", channel: "Slack" } },
    ],
  ),
  t("marketing-social-calendar", "social_calendar_draft", "marketing",
    "Two-week social calendar — themes, copy variants, scheduling slots.",
    "approval-required",
    [
      { id: "s1", kind: "trigger", name: "Bi-weekly cron", description: "Drafts the next 2 weeks' social posts." },
      { id: "s2", kind: "data", name: "Pull brand themes + product news", description: "Campaign calendar + changelog + sales asks." },
      { id: "s3", kind: "skill_invoke", name: "Draft copy variants", description: "3 variants per slot, channel-specific length + tone." },
      { id: "s4", kind: "approval", name: "Brand lead approves", description: "Edit + schedule via approval modal.", approvalGate: { reason: "Brand voice consistency.", channel: "Slack" } },
      { id: "s5", kind: "tool_call", name: "Schedule via Buffer/Sprout", description: "Auto-publish on approved slots." },
    ],
  ),
  t("marketing-brand-voice-qa", "brand_voice_qa", "marketing",
    "Scan outbound copy (emails, landing pages, social) against brand voice guide.",
    "approval-required",
    [
      { id: "s1", kind: "trigger", name: "Pre-publish hook", description: "Any outbound asset draft fires the check." },
      { id: "s2", kind: "skill_invoke", name: "Voice + tone scan", description: "Against brand voice rubric; produce delta report." },
      { id: "s3", kind: "decision", name: "Off-brand?", description: "If score <0.8 → request rewrite. Else continue." },
      { id: "s4", kind: "approval", name: "Brand lead OKs exceptions", description: "Off-brand approvals must be explicit.", approvalGate: { reason: "Brand consistency.", channel: "Slack" } },
    ],
  ),
  t("marketing-lead-enrichment", "lead_enrichment", "marketing",
    "Enrich form-fill leads with firmographics + intent signals before MQL handoff.",
    "autonomous",
    [
      { id: "s1", kind: "trigger", name: "Form-fill webhook", description: "Marketo / HubSpot form-fill event." },
      { id: "s2", kind: "tool_call", name: "Enrich via vendor", description: "Clearbit / ZoomInfo / 6sense pull." },
      { id: "s3", kind: "skill_invoke", name: "Score + tag", description: "ICP fit + intent score + persona tag." },
      { id: "s4", kind: "data", name: "Update CRM + route", description: "Score >threshold = MQL queue; else nurture." },
    ],
  ),
  t("marketing-campaign-perf-digest", "campaign_perf_digest", "marketing",
    "Bi-weekly campaign performance digest — spend, conversions, CAC, recommendations.",
    "autonomous",
    [
      { id: "s1", kind: "trigger", name: "Bi-weekly cron", description: "Every other Friday morning." },
      { id: "s2", kind: "data", name: "Pull spend + conversions", description: "Ad platforms + GA + CRM closed-loop." },
      { id: "s3", kind: "skill_invoke", name: "Analyze + recommend", description: "Best-performing channels + reallocation suggestions." },
      { id: "s4", kind: "notify", name: "Email + post #marketing", description: "Digest with charts." },
    ],
  ),
  t("marketing-content-brief-draft", "content_brief_draft", "marketing",
    "Convert a keyword into a structured content brief — outline, target intent, internal links.",
    "approval-required",
    [
      { id: "s1", kind: "trigger", name: "Keyword added to queue", description: "SEO sheet adds a new target keyword." },
      { id: "s2", kind: "skill_invoke", name: "Research + outline", description: "Search-intent + SERP scan + competitor angles + outline." },
      { id: "s3", kind: "approval", name: "Editor signs off", description: "Editor approves before writer assignment.", approvalGate: { reason: "Editorial standard.", channel: "Slack" } },
      { id: "s4", kind: "tool_call", name: "Open writer assignment", description: "Notion task + Slack DM to writer pool." },
    ],
  ),
  t("marketing-seo-gap-audit", "seo_gap_audit", "marketing",
    "Monthly: where competitors rank and we don't, ranked by traffic potential.",
    "autonomous",
    [
      { id: "s1", kind: "trigger", name: "1st-of-month cron", description: "Monthly SEO audit." },
      { id: "s2", kind: "tool_call", name: "Pull rank data", description: "Ahrefs / Semrush API — competitor keyword universe." },
      { id: "s3", kind: "skill_invoke", name: "Rank gaps by opportunity", description: "Volume × intent × difficulty × business fit." },
      { id: "s4", kind: "notify", name: "Email SEO team", description: "Top-20 gap list as a doc + tracker update." },
    ],
  ),

  // ── Operations (4) ─────────────────────────────────────────────────────
  t("ops-vendor-risk-review", "vendor_risk_review", "ops",
    "Quarterly vendor-risk audit — SOC2, DPA, financial health, alternative options.",
    "approval-required",
    [
      { id: "s1", kind: "trigger", name: "Quarterly cron", description: "Q1/Q2/Q3/Q4 vendor review." },
      { id: "s2", kind: "data", name: "Pull vendor list + posture", description: "Vendor inventory + SOC2/ISO + DPA + contract terms." },
      { id: "s3", kind: "skill_invoke", name: "Risk-score + flag", description: "RAG score per vendor + concrete risks." },
      { id: "s4", kind: "approval", name: "Ops + legal review", description: "Joint review before any action.", approvalGate: { reason: "Vendor decisions impact contracts + security.", channel: "Slack" } },
    ],
  ),
  t("ops-runbook-freshness-check", "runbook_freshness_check", "ops",
    "Audit runbooks for staleness — last-updated, last-tested, on-call usage.",
    "autonomous",
    [
      { id: "s1", kind: "trigger", name: "Monthly cron", description: "1st-of-month runbook health pass." },
      { id: "s2", kind: "data", name: "Pull runbook metadata", description: "Last-updated, linked alert, last-incident-use." },
      { id: "s3", kind: "skill_invoke", name: "Flag stale + unused", description: "Either ≥90d untouched or never linked to an alert." },
      { id: "s4", kind: "notify", name: "Open Linear tickets", description: "Per-runbook ownership-team ticket + Slack ping." },
    ],
  ),
  t("ops-on-call-handoff-brief", "on_call_handoff_brief", "ops",
    "End-of-shift on-call brief — open incidents, watch-items, dashboards-to-watch.",
    "autonomous",
    [
      { id: "s1", kind: "trigger", name: "Shift-rotation cron", description: "Per on-call shift end." },
      { id: "s2", kind: "data", name: "Pull incidents + alerts + tickets", description: "PagerDuty + alert manager + open tickets." },
      { id: "s3", kind: "skill_invoke", name: "Compose handoff", description: "What happened, what to watch, what's still open." },
      { id: "s4", kind: "notify", name: "Slack handoff thread", description: "Posted to #oncall with @mentions." },
    ],
  ),
  t("ops-sla-breach-alert", "sla_breach_alert", "ops",
    "Proactive SLA-breach warning — fires when usage trajectory will breach within 1h.",
    "autonomous",
    [
      { id: "s1", kind: "trigger", name: "5-min metric poll", description: "Polls SLI dashboards every 5 minutes." },
      { id: "s2", kind: "skill_invoke", name: "Trajectory forecast", description: "Will current trajectory breach SLA in next hour?" },
      { id: "s3", kind: "decision", name: "Breach risk >50%?", description: "Yes → alert before customer-visible." },
      { id: "s4", kind: "notify", name: "Page on-call + #incidents", description: "Slack + PagerDuty + auto-open incident channel." },
    ],
  ),

  // ── Engineering (4) ────────────────────────────────────────────────────
  t("eng-pr-review-helper", "pr_review_helper", "engineering",
    "PR opened → smell-test, lint summary, suggested reviewers, risk score.",
    "autonomous",
    [
      { id: "s1", kind: "trigger", name: "PR opened on main repo", description: "GitHub webhook on PR open." },
      { id: "s2", kind: "skill_invoke", name: "Analyze diff", description: "LOC, files, blast radius, smells, missing tests." },
      { id: "s3", kind: "skill_invoke", name: "Suggest reviewers", description: "CODEOWNERS + git-blame heat-map + recent activity." },
      { id: "s4", kind: "notify", name: "Comment on PR", description: "Bot comment with summary + reviewer suggestions." },
    ],
  ),
  t("eng-incident-postmortem-draft", "incident_postmortem_draft", "engineering",
    "Post-incident: draft a postmortem from the war-room Slack + alerts + commits.",
    "approval-required",
    [
      { id: "s1", kind: "trigger", name: "Incident closed", description: "PagerDuty incident-resolve event." },
      { id: "s2", kind: "data", name: "Pull war-room + alerts + diffs", description: "Slack thread + alert timeline + commits + rollbacks." },
      { id: "s3", kind: "skill_invoke", name: "Draft postmortem", description: "Timeline, contributing factors, action items, lessons." },
      { id: "s4", kind: "approval", name: "IC reviews + edits", description: "Incident commander finalizes before publish.", approvalGate: { reason: "Postmortems must be accurate; IC owns the narrative.", channel: "Slack" } },
      { id: "s5", kind: "tool_call", name: "Publish to wiki + ticket actions", description: "Confluence page + Linear tickets for action items." },
    ],
  ),
  t("eng-dependency-audit", "dependency_audit", "engineering",
    "Weekly: outdated dependencies, security advisories, license drift.",
    "autonomous",
    [
      { id: "s1", kind: "trigger", name: "Monday 6am cron", description: "Weekly dep audit." },
      { id: "s2", kind: "tool_call", name: "Run scanners", description: "Dependabot / Renovate / Snyk." },
      { id: "s3", kind: "skill_invoke", name: "Prioritize + group", description: "By severity, blast radius, and PR ergonomics." },
      { id: "s4", kind: "notify", name: "Open grouped PR", description: "Group safe bumps; ping owners on risky ones." },
    ],
  ),
  t("eng-dora-metrics-digest", "dora_metrics_digest", "engineering",
    "Bi-weekly DORA digest — deploy freq, lead time, MTTR, change-failure rate.",
    "autonomous",
    [
      { id: "s1", kind: "trigger", name: "Bi-weekly cron", description: "Every other Friday." },
      { id: "s2", kind: "data", name: "Pull deploys + incidents + PRs", description: "CI + PagerDuty + GitHub." },
      { id: "s3", kind: "skill_invoke", name: "Compute DORA + commentary", description: "4 DORA metrics + narrative on changes." },
      { id: "s4", kind: "notify", name: "Post to #eng-leads", description: "Slack post with trend charts." },
    ],
  ),

  // ── Compliance (4) ─────────────────────────────────────────────────────
  t("compliance-evidence-collect", "evidence_collect", "compliance",
    "Quarterly SOC2/ISO evidence collection — screenshots, configs, ticket exports.",
    "approval-required",
    [
      { id: "s1", kind: "trigger", name: "Quarterly cron", description: "Q1/Q2/Q3/Q4 evidence pass." },
      { id: "s2", kind: "data", name: "Collect from sources", description: "GitHub + IdP + cloud configs + ticket exports." },
      { id: "s3", kind: "skill_invoke", name: "Map evidence → controls", description: "Each artifact tagged to a control + gap list." },
      { id: "s4", kind: "approval", name: "Compliance lead reviews", description: "Approve evidence package before auditor handoff.", approvalGate: { reason: "Auditor-visible material.", channel: "Slack" } },
      { id: "s5", kind: "tool_call", name: "Upload to GRC tool", description: "Vanta / Drata / Tugboat upload." },
    ],
  ),
  t("compliance-access-review", "access_review", "compliance",
    "Quarterly access review — who has access to what, owner approves still-required.",
    "approval-required",
    [
      { id: "s1", kind: "trigger", name: "Quarterly cron", description: "Per SOC2 access-review cadence." },
      { id: "s2", kind: "data", name: "Pull IdP + system access", description: "Okta + GitHub + cloud roles + ticket-system roles." },
      { id: "s3", kind: "skill_invoke", name: "Group by owner", description: "Each access entry grouped under its owner." },
      { id: "s4", kind: "approval", name: "Owners certify or revoke", description: "Bulk-approve / revoke via approval UI.", approvalGate: { reason: "SOC2 CC6 control evidence.", channel: "Slack" } },
      { id: "s5", kind: "tool_call", name: "Revoke + log", description: "Auto-revoke flagged access + audit trail." },
    ],
  ),
  t("compliance-dpa-check", "dpa_check", "compliance",
    "New-vendor DPA sanity check — sub-processors, data-residency, breach SLA.",
    "approval-required",
    [
      { id: "s1", kind: "trigger", name: "New vendor flag", description: "Procurement flags a new data processor." },
      { id: "s2", kind: "skill_invoke", name: "DPA term check", description: "Sub-processor list, data flows, residency, breach-notify window." },
      { id: "s3", kind: "approval", name: "DPO / legal approves", description: "Joint sign-off.", approvalGate: { reason: "GDPR + customer DPA flow-down.", channel: "Slack" } },
    ],
  ),
  t("compliance-policy-attestation", "policy_attestation_check", "compliance",
    "Annual policy-attestation drive — track who's read/acknowledged required policies.",
    "autonomous",
    [
      { id: "s1", kind: "trigger", name: "Annual cron + new-hire trigger", description: "Once a year + on every new hire." },
      { id: "s2", kind: "tool_call", name: "Send attestation links", description: "Per-employee acknowledgement link." },
      { id: "s3", kind: "data", name: "Track completion", description: "Status: pending / acknowledged / overdue." },
      { id: "s4", kind: "notify", name: "Nudge laggards", description: "Reminder ladder; escalate to manager after 14d." },
    ],
  ),
];

export const TEMPLATE_COUNT = WORKFLOW_TEMPLATES.length;

export function getTemplate(id: string): WorkflowTemplate | undefined {
  return WORKFLOW_TEMPLATES.find((tpl) => tpl.id === id);
}

export function getTemplatesByVertical(): Record<TemplateVertical, WorkflowTemplate[]> {
  const out: Partial<Record<TemplateVertical, WorkflowTemplate[]>> = {};
  for (const tpl of WORKFLOW_TEMPLATES) {
    (out[tpl.vertical] ??= []).push(tpl);
  }
  return out as Record<TemplateVertical, WorkflowTemplate[]>;
}

export const TEMPLATE_VERTICALS: TemplateVertical[] = [
  "sales",
  "support",
  "finance",
  "people-ops",
  "marketing",
  "ops",
  "engineering",
  "compliance",
];
