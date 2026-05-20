"use client";

import Link from "next/link";
import { useMemo, useState } from "react";
import { ArrowRight, Search, Clock, ShieldCheck, Sparkles } from "lucide-react";

/**
 * Workflow templates gallery — `/workflows/templates`.
 *
 * Curated catalogue of 50 brand-loud workflow templates spanning
 * Sales, Support, Finance, People Ops, Marketing, Ops, Engineering
 * and Compliance. Closing the n8n template-moat gap: ship-ready
 * plain-English playbooks an operator can adopt in minutes.
 *
 * Each card links to the existing `/workflows/<vertical>/<name>/build`
 * route which already handles the seed-from-template flow.
 */

type Risk = "approval-required" | "autonomous";

type Template = {
  name: string;
  vertical: VerticalId;
  description: string;
  steps: number;
  risk: Risk;
  timeToDeploy: string;
};

type VerticalId =
  | "sales"
  | "support"
  | "finance"
  | "people-ops"
  | "marketing"
  | "ops"
  | "engineering"
  | "compliance";

const VERTICAL_LABEL: Record<VerticalId, string> = {
  sales: "Sales",
  support: "Support",
  finance: "Finance",
  "people-ops": "People Ops",
  marketing: "Marketing",
  ops: "Ops",
  engineering: "Engineering",
  compliance: "Compliance",
};

const RISK_LABEL: Record<Risk, string> = {
  "approval-required": "Approval Required",
  autonomous: "Autonomous",
};

const RISK_COLOR: Record<Risk, string> = {
  "approval-required": "#f59e0b",
  autonomous: "#14b8a6",
};

const TEMPLATES: Template[] = [
  // SALES (7)
  {
    name: "qualify_lead",
    vertical: "sales",
    description: "BANT-style qualification plus calendar invite when the lead clears the bar.",
    steps: 7,
    risk: "approval-required",
    timeToDeploy: "5 min",
  },
  {
    name: "draft_outreach",
    vertical: "sales",
    description: "Personalize outreach using enrichment signals — firmographics, tech stack, recent moves.",
    steps: 6,
    risk: "approval-required",
    timeToDeploy: "8 min",
  },
  {
    name: "book_demo",
    vertical: "sales",
    description: "Detect high-intent signals and propose three demo slots in one reply.",
    steps: 5,
    risk: "autonomous",
    timeToDeploy: "4 min",
  },
  {
    name: "handle_inbound_form",
    vertical: "sales",
    description: "Score, route, and reply to every web form fill within sixty seconds.",
    steps: 6,
    risk: "autonomous",
    timeToDeploy: "6 min",
  },
  {
    name: "dunning_followup",
    vertical: "sales",
    description: "Polite past-due reminders that nudge the buyer without killing the deal.",
    steps: 5,
    risk: "approval-required",
    timeToDeploy: "7 min",
  },
  {
    name: "deal_alert_summary",
    vertical: "sales",
    description: "Daily Slack DM of deals that need a nudge from the rep this week.",
    steps: 4,
    risk: "autonomous",
    timeToDeploy: "3 min",
  },
  {
    name: "competitive_intel_brief",
    vertical: "sales",
    description: "Watch for competitor mentions in CRM notes and synthesize a weekly intel brief.",
    steps: 6,
    risk: "autonomous",
    timeToDeploy: "9 min",
  },

  // SUPPORT (7)
  {
    name: "triage_ticket",
    vertical: "support",
    description: "Classify P0..P3 plus intent, deflect with KB articles, and escalate the rest.",
    steps: 3,
    risk: "autonomous",
    timeToDeploy: "3 min",
  },
  {
    name: "approve_refund",
    vertical: "support",
    description: "Verify refund eligibility, check the ledger, route anything high-value to a manager.",
    steps: 5,
    risk: "approval-required",
    timeToDeploy: "6 min",
  },
  {
    name: "churn_signal_alert",
    vertical: "support",
    description: "Flag at-risk customers from support sentiment trends across the quarter.",
    steps: 5,
    risk: "autonomous",
    timeToDeploy: "8 min",
  },
  {
    name: "kb_freshness_check",
    vertical: "support",
    description: "Find KB articles that haven't been touched or used in tickets this quarter.",
    steps: 4,
    risk: "autonomous",
    timeToDeploy: "4 min",
  },
  {
    name: "escalate_vip",
    vertical: "support",
    description: "Auto-route any VIP ticket to a human within thirty seconds, on-call paged.",
    steps: 4,
    risk: "autonomous",
    timeToDeploy: "5 min",
  },
  {
    name: "weekly_report",
    vertical: "support",
    description: "Auto-summarize the week's tickets — themes, repeat issues, SLA hits — for the support lead.",
    steps: 5,
    risk: "autonomous",
    timeToDeploy: "5 min",
  },
  {
    name: "duplicate_ticket_merge",
    vertical: "support",
    description: "Detect and merge duplicate inbound tickets before they fan out to multiple agents.",
    steps: 4,
    risk: "approval-required",
    timeToDeploy: "6 min",
  },

  // FINANCE (7)
  {
    name: "extract_invoice",
    vertical: "finance",
    description: "OCR plus line-item extraction plus vendor reconcile — drop-in for AP teams.",
    steps: 4,
    risk: "autonomous",
    timeToDeploy: "4 min",
  },
  {
    name: "vendor_reconcile",
    vertical: "finance",
    description: "Match invoices against POs and flag any line-item or quantity discrepancies.",
    steps: 5,
    risk: "approval-required",
    timeToDeploy: "7 min",
  },
  {
    name: "expense_audit",
    vertical: "finance",
    description: "Flag any expense report that falls outside policy — category, amount, or receipt.",
    steps: 4,
    risk: "approval-required",
    timeToDeploy: "5 min",
  },
  {
    name: "cash_runway_alert",
    vertical: "finance",
    description: "Daily Slack ping of cash burn versus runway with a 90-day projection trend.",
    steps: 3,
    risk: "autonomous",
    timeToDeploy: "4 min",
  },
  {
    name: "board_metrics_digest",
    vertical: "finance",
    description: "Compile monthly board KPIs into a single polished digest, ready for review.",
    steps: 6,
    risk: "approval-required",
    timeToDeploy: "10 min",
  },
  {
    name: "ap_aging_summary",
    vertical: "finance",
    description: "Past-due payables roll-up by vendor, age bucket, and amount outstanding.",
    steps: 4,
    risk: "autonomous",
    timeToDeploy: "4 min",
  },
  {
    name: "ar_collections_followup",
    vertical: "finance",
    description: "Polite-then-firm collections workflow with escalation gates at thirty and sixty days.",
    steps: 6,
    risk: "approval-required",
    timeToDeploy: "8 min",
  },

  // PEOPLE OPS (7)
  {
    name: "screen_candidate",
    vertical: "people-ops",
    description: "First-pass screen plus skill match plus interview booking — all in one shot.",
    steps: 8,
    risk: "autonomous",
    timeToDeploy: "9 min",
  },
  {
    name: "exit_interview_synth",
    vertical: "people-ops",
    description: "Quarterly synthesis of exit-interview transcripts into a retention report.",
    steps: 3,
    risk: "approval-required",
    timeToDeploy: "5 min",
  },
  {
    name: "onboarding_check",
    vertical: "people-ops",
    description: "Day-1, 7, 30, 60 check-ins with new hires — feedback collected, signals raised.",
    steps: 5,
    risk: "autonomous",
    timeToDeploy: "6 min",
  },
  {
    name: "performance_review_prep",
    vertical: "people-ops",
    description: "Compile peer and manager feedback ahead of the review cycle, structured by competency.",
    steps: 6,
    risk: "approval-required",
    timeToDeploy: "8 min",
  },
  {
    name: "comp_band_audit",
    vertical: "people-ops",
    description: "Flag any salary that falls outside the published band for the role and level.",
    steps: 4,
    risk: "approval-required",
    timeToDeploy: "5 min",
  },
  {
    name: "referral_followup",
    vertical: "people-ops",
    description: "Track and acknowledge employee referrals — status, bonus eligibility, thank-you note.",
    steps: 4,
    risk: "autonomous",
    timeToDeploy: "4 min",
  },
  {
    name: "offer_letter_compose",
    vertical: "people-ops",
    description: "Generate offer letters from the approved comp ladder, ready for legal review.",
    steps: 5,
    risk: "approval-required",
    timeToDeploy: "6 min",
  },

  // MARKETING (7)
  {
    name: "draft_status_report",
    vertical: "marketing",
    description: "Weekly client status report — campaign perf plus content shipped plus next-week asks.",
    steps: 4,
    risk: "approval-required",
    timeToDeploy: "5 min",
  },
  {
    name: "social_calendar",
    vertical: "marketing",
    description: "Thirty-day social content calendar across channels, with copy and hooks drafted.",
    steps: 6,
    risk: "approval-required",
    timeToDeploy: "10 min",
  },
  {
    name: "brand_voice_qa",
    vertical: "marketing",
    description: "Audit recent content against the brand-voice rubric — flag drift, suggest rewrites.",
    steps: 4,
    risk: "autonomous",
    timeToDeploy: "5 min",
  },
  {
    name: "lead_enrichment",
    vertical: "marketing",
    description: "Add firmographic and technographic data to every inbound lead, ready for routing.",
    steps: 4,
    risk: "autonomous",
    timeToDeploy: "4 min",
  },
  {
    name: "campaign_perf_digest",
    vertical: "marketing",
    description: "Daily Slack digest of paid and organic performance — CTR, CPL, attribution.",
    steps: 5,
    risk: "autonomous",
    timeToDeploy: "6 min",
  },
  {
    name: "influencer_outreach",
    vertical: "marketing",
    description: "Identify relevant influencers from a topic and draft personalized outreach.",
    steps: 6,
    risk: "approval-required",
    timeToDeploy: "9 min",
  },
  {
    name: "press_release_draft",
    vertical: "marketing",
    description: "Turn a product changelog into a polished press release, ready for PR review.",
    steps: 5,
    risk: "approval-required",
    timeToDeploy: "7 min",
  },

  // OPS (4) — splitting Ops/Engineering for proper 9-vertical taxonomy
  {
    name: "vendor_renewal_alert",
    vertical: "ops",
    description: "Sixty-day-out renewal reminders for every active vendor contract.",
    steps: 4,
    risk: "autonomous",
    timeToDeploy: "4 min",
  },
  {
    name: "contract_clause_check",
    vertical: "ops",
    description: "Flag risky language in inbound contracts — auto-renew, liability caps, exclusivity.",
    steps: 5,
    risk: "approval-required",
    timeToDeploy: "7 min",
  },
  {
    name: "meeting_action_extract",
    vertical: "ops",
    description: "Parse meeting recordings into action items with owners and due dates.",
    steps: 4,
    risk: "autonomous",
    timeToDeploy: "5 min",
  },
  {
    name: "weekly_priorities_digest",
    vertical: "ops",
    description: "Compile each team's weekly priorities for the leadership team in one digest.",
    steps: 5,
    risk: "autonomous",
    timeToDeploy: "6 min",
  },

  // ENGINEERING (3)
  {
    name: "incident_response_intake",
    vertical: "engineering",
    description: "Triage P0/P1 incidents, page on-call, and open the war-room channel automatically.",
    steps: 6,
    risk: "autonomous",
    timeToDeploy: "8 min",
  },
  {
    name: "post_mortem_compose",
    vertical: "engineering",
    description: "Compile timeline plus five-whys from the Slack incident channel — ready for review.",
    steps: 6,
    risk: "approval-required",
    timeToDeploy: "10 min",
  },
  {
    name: "runbook_freshness",
    vertical: "engineering",
    description: "Find runbooks that haven't been opened or rehearsed in six months and ping owners.",
    steps: 4,
    risk: "autonomous",
    timeToDeploy: "4 min",
  },

  // COMPLIANCE (8)
  {
    name: "gdpr_dsar_handle",
    vertical: "compliance",
    description: "Process GDPR Data Subject Access Requests — verify, collect, redact, deliver.",
    steps: 7,
    risk: "approval-required",
    timeToDeploy: "12 min",
  },
  {
    name: "audit_evidence_pack",
    vertical: "compliance",
    description: "Compile the evidence pack for SOC 2 and ISO 27001 quarterly reviews.",
    steps: 6,
    risk: "approval-required",
    timeToDeploy: "10 min",
  },
  {
    name: "vendor_risk_score",
    vertical: "compliance",
    description: "Score new vendors against the security questionnaire and flag any red items.",
    steps: 5,
    risk: "approval-required",
    timeToDeploy: "8 min",
  },
  {
    name: "access_review_quarterly",
    vertical: "compliance",
    description: "Run the quarterly access review against RBAC — flag drift and over-permissioned roles.",
    steps: 6,
    risk: "approval-required",
    timeToDeploy: "9 min",
  },
  {
    name: "policy_drift_detect",
    vertical: "compliance",
    description: "Flag any policy not updated in twelve months and nudge the policy owner.",
    steps: 4,
    risk: "autonomous",
    timeToDeploy: "5 min",
  },
  {
    name: "training_compliance",
    vertical: "compliance",
    description: "Track and nudge required training completion — escalate at the deadline.",
    steps: 5,
    risk: "autonomous",
    timeToDeploy: "6 min",
  },
  {
    name: "breach_notification_draft",
    vertical: "compliance",
    description: "Compose breach notification drafts with the legal review gate baked in.",
    steps: 6,
    risk: "approval-required",
    timeToDeploy: "11 min",
  },
  {
    name: "hipaa_audit_log",
    vertical: "compliance",
    description: "Daily review of access patterns against HIPAA requirements — flag anomalies.",
    steps: 5,
    risk: "autonomous",
    timeToDeploy: "7 min",
  },
];

const VERTICAL_FILTERS: { id: VerticalId | "all"; label: string }[] = [
  { id: "all", label: "All" },
  { id: "sales", label: "Sales" },
  { id: "support", label: "Support" },
  { id: "finance", label: "Finance" },
  { id: "people-ops", label: "People Ops" },
  { id: "marketing", label: "Marketing" },
  { id: "ops", label: "Ops" },
  { id: "engineering", label: "Engineering" },
  { id: "compliance", label: "Compliance" },
];

const RISK_FILTERS: { id: Risk | "all"; label: string }[] = [
  { id: "all", label: "All" },
  { id: "autonomous", label: "Autonomous" },
  { id: "approval-required", label: "Approval-required" },
];

export default function WorkflowTemplatesPage() {
  const [query, setQuery] = useState("");
  const [vertical, setVertical] = useState<VerticalId | "all">("all");
  const [risk, setRisk] = useState<Risk | "all">("all");

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    return TEMPLATES.filter((t) => {
      if (vertical !== "all" && t.vertical !== vertical) return false;
      if (risk !== "all" && t.risk !== risk) return false;
      if (!q) return true;
      return (
        t.name.toLowerCase().includes(q) ||
        t.description.toLowerCase().includes(q) ||
        VERTICAL_LABEL[t.vertical].toLowerCase().includes(q) ||
        t.vertical.toLowerCase().includes(q)
      );
    });
  }, [query, vertical, risk]);

  return (
    <>
      <link rel="preconnect" href="https://fonts.googleapis.com" />
      <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
      <link
        href="https://fonts.googleapis.com/css2?family=Audiowide&family=Inter:wght@200;300;400;500;600;700&family=JetBrains+Mono:wght@400;500;600;700&display=swap"
        rel="stylesheet"
      />
      <style jsx global>{`
        .tpl-stage {
          padding: 48px 32px 96px;
          max-width: 1240px;
          margin: 0 auto;
          background: #fafaff;
          color: #1e1b4b;
        }
        .tpl-eyebrow {
          display: inline-flex;
          align-items: center;
          gap: 8px;
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: #71717a;
          letter-spacing: 0.2em;
          text-transform: uppercase;
          font-weight: 700;
          margin-bottom: 18px;
          padding: 5px 14px;
          border-radius: 999px;
          border: 1px solid #ece9f5;
          background: #ffffff;
        }
        .tpl-eyebrow-dot {
          width: 6px;
          height: 6px;
          border-radius: 50%;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          box-shadow: 0 0 6px rgba(139, 92, 246, 0.6);
        }
        .tpl-hero {
          font-family: "Audiowide", "Inter", sans-serif;
          font-size: 56px;
          line-height: 1.05;
          font-weight: 400;
          letter-spacing: 0;
          margin: 0 0 14px;
          color: #1e1b4b;
        }
        .tpl-hero strong {
          font-weight: 400;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          -webkit-background-clip: text;
          background-clip: text;
          color: transparent;
        }
        .tpl-sub {
          font-family: "Inter", sans-serif;
          font-size: 17px;
          font-weight: 300;
          color: #3f3f46;
          margin: 0 0 32px;
          max-width: 680px;
          line-height: 1.55;
        }
        .tpl-search-row {
          display: flex;
          align-items: center;
          gap: 14px;
          padding: 16px 20px;
          border-radius: 14px;
          border: 1px solid #ece9f5;
          background: white;
          margin-bottom: 18px;
          box-shadow: 0 1px 0 rgba(0, 0, 0, 0.02);
        }
        .tpl-search-icon {
          width: 28px;
          height: 28px;
          border-radius: 8px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
          display: grid;
          place-items: center;
          flex-shrink: 0;
        }
        .tpl-search-input {
          flex: 1;
          border: none;
          outline: none;
          font-family: "Inter", sans-serif;
          font-size: 18px;
          color: #1e1b4b;
          background: transparent;
        }
        .tpl-search-kbd {
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: #71717a;
          padding: 4px 8px;
          border-radius: 6px;
          border: 1px solid #ece9f5;
          background: #fafaff;
          letter-spacing: 0.04em;
        }
        .tpl-count-pill {
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: #71717a;
          letter-spacing: 0.08em;
          font-weight: 600;
        }
        .tpl-filter-row {
          display: flex;
          flex-wrap: wrap;
          gap: 8px;
          margin-bottom: 14px;
          align-items: center;
        }
        .tpl-filter-label {
          font-family: "JetBrains Mono", monospace;
          font-size: 10.5px;
          color: #71717a;
          letter-spacing: 0.18em;
          text-transform: uppercase;
          font-weight: 700;
          margin-right: 6px;
        }
        .tpl-filter-pill {
          padding: 7px 14px;
          border-radius: 999px;
          background: white;
          border: 1px solid #ece9f5;
          color: #3f3f46;
          font-family: "Inter", sans-serif;
          font-size: 13px;
          font-weight: 500;
          cursor: pointer;
          transition: all 0.14s;
        }
        .tpl-filter-pill:hover {
          border-color: rgba(139, 92, 246, 0.4);
          color: #1e1b4b;
        }
        .tpl-filter-pill[data-active="true"] {
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
          border-color: transparent;
          box-shadow: 0 4px 12px -4px rgba(139, 92, 246, 0.5);
        }
        .tpl-risk-row {
          display: flex;
          gap: 8px;
          margin-bottom: 32px;
          align-items: center;
        }
        .tpl-section-label {
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: #71717a;
          letter-spacing: 0.16em;
          text-transform: uppercase;
          font-weight: 700;
          margin-bottom: 16px;
          display: flex;
          align-items: center;
          justify-content: space-between;
        }
        .tpl-section-label-accent {
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          -webkit-background-clip: text;
          background-clip: text;
          color: transparent;
        }
        .tpl-grid {
          display: grid;
          grid-template-columns: repeat(3, 1fr);
          gap: 18px;
        }
        @media (max-width: 1100px) {
          .tpl-grid {
            grid-template-columns: repeat(2, 1fr);
          }
        }
        @media (max-width: 720px) {
          .tpl-grid {
            grid-template-columns: 1fr;
          }
        }
        .tpl-card {
          background: white;
          border: 1px solid #ece9f5;
          border-radius: 16px;
          padding: 22px;
          transition: all 0.18s;
          position: relative;
          overflow: hidden;
          color: inherit;
          text-decoration: none;
          display: flex;
          flex-direction: column;
          min-height: 260px;
        }
        .tpl-card:hover {
          border-color: rgba(139, 92, 246, 0.4);
          transform: translateY(-3px);
          box-shadow: 0 16px 32px -14px rgba(139, 92, 246, 0.22);
        }
        .tpl-card-top {
          display: flex;
          justify-content: space-between;
          align-items: flex-start;
          margin-bottom: 14px;
          gap: 12px;
        }
        .tpl-card-vertical {
          font-family: "JetBrains Mono", monospace;
          font-size: 10.5px;
          color: #8b5cf6;
          letter-spacing: 0.14em;
          text-transform: uppercase;
          font-weight: 700;
        }
        .tpl-card-risk {
          font-family: "JetBrains Mono", monospace;
          font-size: 9.5px;
          letter-spacing: 0.1em;
          text-transform: uppercase;
          font-weight: 700;
          padding: 3px 9px;
          border-radius: 999px;
          color: white;
          white-space: nowrap;
          flex-shrink: 0;
        }
        .tpl-card-name {
          font-family: "Inter", sans-serif;
          font-weight: 600;
          font-size: 20px;
          color: #1e1b4b;
          margin: 0 0 8px;
          letter-spacing: -0.01em;
        }
        .tpl-card-desc {
          font-family: "Inter", sans-serif;
          font-size: 13.5px;
          color: #3f3f46;
          line-height: 1.55;
          margin: 0 0 18px;
        }
        .tpl-card-steps {
          display: flex;
          gap: 10px;
          align-items: center;
          margin-bottom: 14px;
        }
        .tpl-step-pill {
          min-width: 36px;
          height: 30px;
          padding: 0 10px;
          display: inline-flex;
          align-items: center;
          justify-content: center;
          border-radius: 8px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
          font-family: "JetBrains Mono", monospace;
          font-size: 12px;
          font-weight: 700;
          letter-spacing: 0.02em;
        }
        .tpl-step-bar {
          flex: 1;
          height: 3px;
          background: linear-gradient(
            90deg,
            #8b5cf6 0%,
            #ec4899 60%,
            rgba(236, 72, 153, 0.15) 100%
          );
          border-radius: 2px;
        }
        .tpl-step-count {
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: #71717a;
          letter-spacing: 0.04em;
          font-weight: 600;
        }
        .tpl-card-meta {
          display: flex;
          flex-wrap: wrap;
          gap: 10px 14px;
          font-family: "JetBrains Mono", monospace;
          font-size: 10.5px;
          color: #71717a;
          letter-spacing: 0.04em;
          text-transform: uppercase;
          font-weight: 600;
          margin-bottom: 18px;
        }
        .tpl-card-meta-item {
          display: inline-flex;
          align-items: center;
          gap: 5px;
        }
        .tpl-card-cta {
          margin-top: auto;
          display: inline-flex;
          align-items: center;
          justify-content: center;
          gap: 8px;
          padding: 11px 16px;
          border-radius: 10px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
          font-family: "Inter", sans-serif;
          font-size: 14px;
          font-weight: 600;
          border: none;
          cursor: pointer;
          text-decoration: none;
          box-shadow: 0 4px 12px -3px rgba(139, 92, 246, 0.4);
          transition: all 0.16s;
        }
        .tpl-card:hover .tpl-card-cta {
          box-shadow: 0 8px 18px -3px rgba(236, 72, 153, 0.45);
        }
        .tpl-empty {
          padding: 64px 24px;
          text-align: center;
          border: 1px dashed #ece9f5;
          border-radius: 16px;
          background: white;
          color: #71717a;
          font-family: "Inter", sans-serif;
        }
        .tpl-empty-icon {
          width: 48px;
          height: 48px;
          margin: 0 auto 14px;
          border-radius: 12px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
          display: grid;
          place-items: center;
        }
        .tpl-empty h3 {
          font-size: 18px;
          color: #1e1b4b;
          margin: 0 0 6px;
          font-weight: 600;
        }
        .tpl-empty p {
          font-size: 14px;
          margin: 0;
        }
        .tpl-clear-btn {
          margin-top: 14px;
          padding: 8px 16px;
          border-radius: 8px;
          background: transparent;
          border: 1px solid #ece9f5;
          color: #8b5cf6;
          font-family: "Inter", sans-serif;
          font-size: 13px;
          font-weight: 500;
          cursor: pointer;
        }
        .tpl-clear-btn:hover {
          border-color: rgba(139, 92, 246, 0.4);
        }
      `}</style>

      <div className="tpl-stage">
        <div className="tpl-eyebrow">
          <span className="tpl-eyebrow-dot" />
          TEMPLATES · CURATED
        </div>

        <h1 className="tpl-hero">
          Hit the ground <strong>running.</strong>
        </h1>

        <p className="tpl-sub">
          50+ pre-built workflows your AI teammates can use today. Pick one, customize in chat,
          ship in minutes.
        </p>

        <div className="tpl-search-row">
          <div className="tpl-search-icon">
            <Search size={16} />
          </div>
          <input
            type="text"
            className="tpl-search-input"
            placeholder="Search templates by name, vertical, or what they do…"
            aria-label="Search templates"
            data-testid="templates-search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <span className="tpl-search-kbd">⌘K</span>
          <span className="tpl-count-pill">
            {filtered.length} of {TEMPLATES.length}
          </span>
        </div>

        <div className="tpl-filter-row" role="group" aria-label="Filter by vertical">
          <span className="tpl-filter-label">VERTICAL</span>
          {VERTICAL_FILTERS.map((f) => (
            <button
              type="button"
              key={f.id}
              className="tpl-filter-pill"
              data-active={vertical === f.id}
              onClick={() => setVertical(f.id)}
              data-testid={`vertical-filter-${f.id}`}
            >
              {f.label}
            </button>
          ))}
        </div>

        <div className="tpl-risk-row" role="group" aria-label="Filter by risk">
          <span className="tpl-filter-label">RISK</span>
          {RISK_FILTERS.map((f) => (
            <button
              type="button"
              key={f.id}
              className="tpl-filter-pill"
              data-active={risk === f.id}
              onClick={() => setRisk(f.id)}
              data-testid={`risk-filter-${f.id}`}
            >
              {f.label}
            </button>
          ))}
        </div>

        <div className="tpl-section-label">
          <span>
            <span className="tpl-section-label-accent">CURATED GALLERY</span> · CLOSE THE TEMPLATE GAP
          </span>
          <span>READY TO SHIP</span>
        </div>

        {filtered.length === 0 ? (
          <div className="tpl-empty">
            <div className="tpl-empty-icon">
              <Sparkles size={20} />
            </div>
            <h3>No templates match those filters.</h3>
            <p>Try widening the vertical, switching risk to All, or clearing the search box.</p>
            <button
              type="button"
              className="tpl-clear-btn"
              onClick={() => {
                setQuery("");
                setVertical("all");
                setRisk("all");
              }}
            >
              Clear filters
            </button>
          </div>
        ) : (
          <div className="tpl-grid">
            {filtered.map((t) => (
              <Link
                key={`${t.vertical}/${t.name}`}
                href={`/workflows/${t.vertical}/${t.name}/build`}
                className="tpl-card"
                data-testid={`tpl-card-${t.vertical}-${t.name}`}
              >
                <div className="tpl-card-top">
                  <span className="tpl-card-vertical">{VERTICAL_LABEL[t.vertical]}</span>
                  <span
                    className="tpl-card-risk"
                    style={{ background: RISK_COLOR[t.risk] }}
                  >
                    {RISK_LABEL[t.risk]}
                  </span>
                </div>

                <h3 className="tpl-card-name">{t.name}</h3>
                <p className="tpl-card-desc">{t.description}</p>

                <div className="tpl-card-steps">
                  <div className="tpl-step-pill">{t.steps}</div>
                  <div className="tpl-step-bar" />
                  <span className="tpl-step-count">{t.steps} steps</span>
                </div>

                <div className="tpl-card-meta">
                  <span className="tpl-card-meta-item">
                    <ShieldCheck size={11} />
                    {t.risk === "approval-required" ? "human-in-loop" : "fully autonomous"}
                  </span>
                  <span className="tpl-card-meta-item">
                    <Clock size={11} /> {t.timeToDeploy} to deploy
                  </span>
                </div>

                <span className="tpl-card-cta">
                  Use this template
                  <ArrowRight size={15} />
                </span>
              </Link>
            ))}
          </div>
        )}
      </div>
    </>
  );
}
