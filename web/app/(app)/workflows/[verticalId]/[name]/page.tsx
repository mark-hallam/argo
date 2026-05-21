"use client";
import "./page.css";

import Link from "next/link";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { Suspense, use, useCallback, useState } from "react";
import {
  ArrowLeft,
  GitBranch,
  Play,
  Pencil,
  CheckCircle2,
  MessageSquare,
  ShieldCheck,
  Sparkles,
  Database,
} from "lucide-react";
import { WorkflowViewerSkeleton } from "@/components/skycot/workflows/WorkflowViewerSkeleton";
import { WorkflowViewerErrorState } from "@/components/skycot/workflows/WorkflowEdgeStates";
import { WorkflowProposeModal } from "@/components/skycot/WorkflowProposeModal";

/**
 * Visual workflow detail viewer — `/workflows/<v>/<name>`.
 *
 * Shows the workflow steps as a vertical timeline of cards instead of
 * raw TOML. Each step renders:
 *   - step icon (kind-specific)
 *   - step name + plain-English description
 *   - "approval gate" badge if gated
 *   - inputs/outputs preview
 *
 * V1 seeds the steps from a curated set keyed by (vertical, name).
 * Live data binding (fetch from gateway via PR #1860 routes) lands in
 * a follow-up brief; the shape here matches the WorkflowStep contract
 * the gateway returns so the swap is mechanical.
 *
 * Operator can edit a step by clicking the pencil; that opens an
 * inline editor (textarea today, structured form in v1.x). Persisting
 * goes through the gateway PUT workflow endpoint.
 */

type StepKind = "trigger" | "skill_invoke" | "approval" | "tool_call" | "notify" | "decision" | "data";

interface WorkflowStep {
  id: string;
  kind: StepKind;
  name: string;
  description: string;
  approvalGate?: { reason: string; channel: string };
  inputs?: string[];
  outputs?: string[];
}

interface WorkflowSpec {
  name: string;
  vertical: string;
  description: string;
  version: string;
  steps: WorkflowStep[];
  risk: "approval-required" | "autonomous";
}

const SEED_WORKFLOWS: Record<string, WorkflowSpec> = {
  "support/triage_ticket": {
    name: "triage_ticket",
    vertical: "support",
    description:
      "Classify inbound tickets, deflect with KB articles, escalate VIPs to a human. Median resolution under 4 minutes.",
    version: "v6",
    risk: "autonomous",
    steps: [
      {
        id: "step-1",
        kind: "trigger",
        name: "Ticket arrives",
        description: "Zendesk, Intercom, Help Scout, or email — all wire to the same trigger.",
        inputs: ["customer_email", "subject", "body", "tags"],
      },
      {
        id: "step-2",
        kind: "skill_invoke",
        name: "Classify severity + intent",
        description:
          "Skill `support.classify` tags severity (P0..P3) + intent (refund / how-to / bug / billing).",
        outputs: ["severity", "intent"],
      },
      {
        id: "step-3",
        kind: "decision",
        name: "VIP / churn-signal branch",
        description:
          "If the customer is VIP, on a renewal-risk plan, or the message contains 'cancel'/'churn', skip self-deflection and hand straight to a human.",
      },
    ],
  },
  "sales/qualify_lead": {
    name: "qualify_lead",
    vertical: "sales",
    description:
      "BANT-style qualification + calendar-link insertion + CRM update on accept.",
    version: "v4",
    risk: "approval-required",
    steps: [
      {
        id: "step-1",
        kind: "trigger",
        name: "Lead from web / paid / referral",
        description:
          "HubSpot/Salesforce webhook fires when a form is submitted or a high-intent page is visited.",
      },
      {
        id: "step-2",
        kind: "skill_invoke",
        name: "Enrich + BANT score",
        description:
          "Skill `lead.enrich` pulls firmographic + technographic data; BANT-scores 0-100 across Budget, Authority, Need, Timeline.",
      },
      {
        id: "step-3",
        kind: "decision",
        name: "Branch on BANT score",
        description: "≥75 = book demo with AE. 50-74 = nurture. <50 = polite decline.",
      },
      {
        id: "step-4",
        kind: "approval",
        name: "AE approves outreach",
        description:
          "For ≥75 leads, the assigned AE gets a Slack DM with the qualification summary + one-tap approve to send the outreach email.",
        approvalGate: {
          reason: "Outreach to enterprise prospects requires the AE's eyes-on signoff.",
          channel: "Slack",
        },
      },
      {
        id: "step-5",
        kind: "tool_call",
        name: "Send personalized outreach + calendar link",
        description:
          "Gmail / Outlook sends the personalized message with the AE's Calendly slot. CRM record auto-updated.",
      },
    ],
  },
  "finance/extract_invoice": {
    name: "extract_invoice",
    vertical: "finance",
    description:
      "Pull line items from PDF or scanned invoices, reconcile vendor against QuickBooks / Xero / NetSuite, flag exceptions.",
    version: "v1",
    risk: "autonomous",
    steps: [
      {
        id: "step-1",
        kind: "trigger",
        name: "Invoice email arrives in AP inbox",
        description: "ap@yourcompany.com forwarded mail or vendor portal upload.",
      },
      {
        id: "step-2",
        kind: "skill_invoke",
        name: "OCR + line-item extraction",
        description:
          "Skill `invoice.extract` runs OCR + structured extraction. Outputs vendor, line items, total, tax.",
        outputs: ["vendor_id", "line_items[]", "total", "tax", "due_date"],
      },
      {
        id: "step-3",
        kind: "tool_call",
        name: "Reconcile against PO + vendor data",
        description: "QuickBooks / Xero / NetSuite match. Flag discrepancy if line item differs from PO by >5%.",
      },
      {
        id: "step-4",
        kind: "decision",
        name: "Auto-approve or flag for review",
        description:
          "Match clean → auto-approve. Discrepancy → flag for AP human review.",
      },
    ],
  },
  "support/approve_refund": {
    name: "approve_refund",
    vertical: "support",
    description:
      "Verify refund eligibility, check ledger, route high-value to manager.",
    version: "v2",
    risk: "approval-required",
    steps: [
      {
        id: "step-1",
        kind: "trigger",
        name: "Refund request from support agent",
        description: "Comes in via Zendesk macro or Intercom workflow.",
      },
      {
        id: "step-2",
        kind: "skill_invoke",
        name: "Verify eligibility",
        description:
          "Skill `refund.verify` checks return window, condition, and refund-policy clauses.",
      },
      {
        id: "step-3",
        kind: "tool_call",
        name: "Check ledger balance",
        description: "Stripe / Shopify ledger pull. Cross-check against customer's lifetime value.",
      },
      {
        id: "step-4",
        kind: "decision",
        name: "Threshold check",
        description: "<$200 = auto-approve. $200-1000 = manager approval. >$1000 = director.",
      },
      {
        id: "step-5",
        kind: "approval",
        name: "Manager approves high-value refund",
        description: "Slack DM with full context, one-tap approve / reject.",
        approvalGate: {
          reason: "Refunds above the auto-approve threshold need manager signoff for revenue tracking.",
          channel: "Slack",
        },
      },
    ],
  },
  "people-ops/exit_interview_synth": {
    name: "exit_interview_synth",
    vertical: "people-ops",
    description:
      "Combine exit-interview transcripts into a confidential quarterly report flagging retention patterns.",
    version: "v2",
    risk: "approval-required",
    steps: [
      {
        id: "step-1",
        kind: "trigger",
        name: "Quarterly cron + transcripts uploaded",
        description: "Calendar-triggered; pulls all signed exit-interview transcripts from the secure store.",
      },
      {
        id: "step-2",
        kind: "skill_invoke",
        name: "Pattern-extract across cohort",
        description:
          "Skill `feedback.synthesize` finds themes (manager, compensation, growth, culture) without naming individuals.",
      },
      {
        id: "step-3",
        kind: "approval",
        name: "Chief People Officer approves the synthesis",
        description: "Confidential synthesis goes to CPO for review before exec circulation.",
        approvalGate: {
          reason: "Exit-interview synthesis is sensitive — CPO eyes-only before exec share.",
          channel: "Email",
        },
      },
    ],
  },
  "marketing/draft_status_report": {
    name: "draft_status_report",
    vertical: "marketing",
    description:
      "Weekly client status report: campaign performance, content shipped, asks for next week.",
    version: "v3",
    risk: "approval-required",
    steps: [
      {
        id: "step-1",
        kind: "trigger",
        name: "Friday 9am cron",
        description: "Fires every Friday at 9am client-local-time.",
      },
      {
        id: "step-2",
        kind: "skill_invoke",
        name: "Pull campaign + content KPIs",
        description: "Google Analytics, Meta Ads, LinkedIn Campaign Manager.",
      },
      {
        id: "step-3",
        kind: "skill_invoke",
        name: "Draft in client's brand voice",
        description:
          "Skill `narrative.compose` writes the report in the client's voice (length, tone, vocabulary all match prior drafts).",
      },
      {
        id: "step-4",
        kind: "approval",
        name: "Account director approves",
        description: "Slack DM with the draft + one-tap edit / send.",
        approvalGate: {
          reason: "Client-facing reports go out under the account director's signature.",
          channel: "Slack",
        },
      },
    ],
  },
  "gtm-engineer/post_listing": {
    name: "post_listing",
    vertical: "gtm-engineer",
    description:
      "When a seller submits inventory, validate it, generate the listing copy, and post to Facebook Marketplace once the seller approves.",
    version: "v3",
    risk: "approval-required",
    steps: [
      {
        id: "step-1",
        kind: "trigger",
        name: "Seller submits inventory",
        description: "Slack DM, email, or web inbox carrying photos + price + condition.",
        inputs: ["photos[]", "price", "condition", "title"],
      },
      {
        id: "step-2",
        kind: "skill_invoke",
        name: "Validate inventory data",
        description:
          "Skill `inventory.validate` checks the price against market range, runs photo OCR for category hints, flags below-floor pricing.",
        inputs: ["seller input"],
        outputs: ["validation report"],
      },
      {
        id: "step-3",
        kind: "decision",
        name: "Branch on validation outcome",
        description:
          "If price is below floor or condition is unverifiable, route to manager review. Otherwise continue.",
      },
      {
        id: "step-4",
        kind: "skill_invoke",
        name: "Generate listing copy",
        description:
          "Skill `listing.compose` writes a 60-word buyer-facing description, picks 3 highlighted features, attaches up to 5 photos.",
        outputs: ["listing.title", "listing.body", "listing.photos[]"],
      },
      {
        id: "step-5",
        kind: "approval",
        name: "Seller approves the listing",
        description:
          "Send draft listing + image carousel to the seller; one-tap approve / request edits.",
        approvalGate: {
          reason: "Posting public listings requires explicit seller sign-off.",
          channel: "Slack",
        },
      },
      {
        id: "step-6",
        kind: "tool_call",
        name: "Post to Facebook Marketplace",
        description:
          "Your teammate's browser opens Facebook Marketplace, fills the form, uploads photos, submits. Returns the public listing URL.",
        outputs: ["listing.url"],
      },
    ],
  },
  "people-ops/screen_candidate": {
    name: "screen_candidate",
    vertical: "people-ops",
    description:
      "First-pass screen of inbound candidates against the JD. Surfaces strong matches, declines clear misses, books interviews for the in-between.",
    version: "v5",
    risk: "autonomous",
    steps: [
      {
        id: "step-1",
        kind: "trigger",
        name: "Candidate applies",
        description: "Resume + cover-letter arrive via Greenhouse webhook or email.",
      },
      {
        id: "step-2",
        kind: "skill_invoke",
        name: "Parse resume + JD",
        description: "Skill `candidate.parse` extracts skills, years, employers; loads the open JD.",
      },
      {
        id: "step-3",
        kind: "skill_invoke",
        name: "Score skill-match",
        description: "0-100 score across hard skills, years, optional must-haves; flags red flags.",
      },
      {
        id: "step-4",
        kind: "decision",
        name: "Branch on score band",
        description: "≥80 = strong yes; 50-79 = book screen; <50 = polite decline.",
      },
      {
        id: "step-5",
        kind: "skill_invoke",
        name: "Compose response",
        description: "Skill `outreach.compose` writes the candidate-facing message in your voice.",
      },
      {
        id: "step-6",
        kind: "tool_call",
        name: "Book interview / send decline",
        description: "Strong yes → calendly invite; book screen → 15-min slots; decline → polite no.",
      },
      {
        id: "step-7",
        kind: "notify",
        name: "Update hiring manager",
        description: "Slack DM the hiring manager with score, summary, next step taken.",
      },
      {
        id: "step-8",
        kind: "data",
        name: "Append to ATS",
        description: "Greenhouse note + tag (strong-yes / phone-screen / declined).",
      },
    ],
  },
};

const STEP_KIND_META: Record<
  StepKind,
  { icon: React.ReactNode; color: string; label: string }
> = {
  trigger: { icon: <Play size={14} />, color: "#10b981", label: "Trigger" },
  skill_invoke: { icon: <Sparkles size={14} />, color: "#8b5cf6", label: "Skill" },
  approval: { icon: <ShieldCheck size={14} />, color: "#f59e0b", label: "Approval gate" },
  tool_call: { icon: <Database size={14} />, color: "#0891b2", label: "Tool call" },
  notify: { icon: <MessageSquare size={14} />, color: "#ec4899", label: "Notify" },
  decision: { icon: <GitBranch size={14} />, color: "#6366f1", label: "Decision" },
  data: { icon: <Database size={14} />, color: "#52525b", label: "Data" },
};

export default function WorkflowDetailPage({
  params,
}: {
  params: Promise<{ verticalId: string; name: string }>;
}) {
  return (
    <Suspense fallback={<WorkflowViewerSkeleton />}>
      <ViewerInner params={params} />
    </Suspense>
  );
}

function ViewerInner({
  params,
}: {
  params: Promise<{ verticalId: string; name: string }>;
}) {
  const { verticalId, name } = use(params);
  const router = useRouter();
  const pathname = usePathname() ?? `/workflows/${verticalId}/${name}`;
  const searchParams = useSearchParams();
  const [editChatOpen, setEditChatOpen] = useState(false);
  // Gateway-fetch-state simulator. See sibling /build page for the
  // rationale: real fetch lands in a follow-up PR; this lets each
  // branch be reached via URL flag for tests + screenshots.
  const stateFlag = searchParams?.get("state") ?? null;
  const key = `${verticalId}/${name}`;
  const workflow = SEED_WORKFLOWS[key];

  const resetState = useCallback(() => {
    const params = new URLSearchParams(searchParams?.toString() ?? "");
    params.delete("state");
    const qs = params.toString();
    router.replace(qs ? `${pathname}?${qs}` : pathname);
  }, [router, pathname, searchParams]);

  if (stateFlag === "loading") {
    return <WorkflowViewerSkeleton />;
  }

  if (stateFlag === "error") {
    return <WorkflowViewerErrorState onRetry={resetState} />;
  }

  if (!workflow) {
    return (
      <div style={{ padding: 48, textAlign: "center", color: "var(--muted-foreground)" }}>
        Workflow not found. Try one from <Link href="/workflows" style={{ color: "#8b5cf6" }}>the catalog</Link>.
      </div>
    );
  }

  return (
    <>
      <link rel="preconnect" href="https://fonts.googleapis.com" />
      <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
      <link
        href="https://fonts.googleapis.com/css2?family=Audiowide&family=JetBrains+Mono:wght@400;500;600;700&display=swap"
        rel="stylesheet"
      />
      <div className="wfd-stage">
        <Link href="/workflows" className="wfd-back">
          <ArrowLeft size={12} />
          ALL WORKFLOWS
        </Link>

        <div className="wfd-card">
          <div className="wfd-meta-row">
            <span className="wfd-vertical">{workflow.vertical}</span>
            <span
              className="wfd-risk"
              style={{
                background:
                  workflow.risk === "approval-required" ? "#f59e0b" : "#14b8a6",
              }}
            >
              {workflow.risk === "approval-required" ? "Approval required" : "Autonomous"}
            </span>
            <span className="wfd-version">version {workflow.version}</span>
          </div>
          <h1 className="wfd-name">{workflow.name}</h1>
          <p className="wfd-desc">{workflow.description}</p>
          <div className="wfd-actions">
            <Link
              href={`/workflows/${workflow.vertical}/${workflow.name}/build`}
              className="wfd-btn wfd-btn-primary"
            >
              <Pencil size={13} /> Open builder
            </Link>
            <button
              type="button"
              className="wfd-btn"
              data-testid="wfd-edit-in-chat"
              onClick={() => setEditChatOpen(true)}
            >
              <MessageSquare size={13} /> Edit in chat
            </button>
            <Link
              href={`/verticals/${workflow.vertical}/workflows/${workflow.name}`}
              className="wfd-btn"
            >
              <GitBranch size={13} /> Version history
            </Link>
            <button type="button" className="wfd-btn">
              <Play size={13} /> Test run
            </button>
          </div>
        </div>

        <div className="wfd-section-label">
          STEPS · {workflow.steps.length} TOTAL
        </div>

        <div className="wfd-timeline">
          {workflow.steps.map((step, idx) => {
            const meta = STEP_KIND_META[step.kind];
            return (
              <div className="wfd-step" key={step.id}>
                <div className="wfd-step-icon" style={{ background: meta.color }}>
                  {meta.icon}
                </div>
                <div className="wfd-step-body">
                  <div className="wfd-step-head">
                    <span
                      className="wfd-step-kind"
                      style={{
                        background: `${meta.color}1a`,
                        color: meta.color,
                      }}
                    >
                      {idx + 1}. {meta.label}
                    </span>
                    <h2 className="wfd-step-name">{step.name}</h2>
                    <button type="button" className="wfd-step-edit" aria-label={`Edit ${step.name}`}>
                      <Pencil size={13} />
                    </button>
                  </div>
                  <p className="wfd-step-desc">{step.description}</p>
                  {step.inputs && step.inputs.length > 0 ? (
                    <div className="wfd-step-meta">
                      <span className="wfd-step-meta-row">
                        <span className="wfd-step-meta-key">In:</span>
                        {step.inputs.join(", ")}
                      </span>
                      {step.outputs && step.outputs.length > 0 ? (
                        <span className="wfd-step-meta-row">
                          <span className="wfd-step-meta-key">Out:</span>
                          {step.outputs.join(", ")}
                        </span>
                      ) : null}
                    </div>
                  ) : step.outputs && step.outputs.length > 0 ? (
                    <div className="wfd-step-meta">
                      <span className="wfd-step-meta-row">
                        <span className="wfd-step-meta-key">Out:</span>
                        {step.outputs.join(", ")}
                      </span>
                    </div>
                  ) : null}
                  {step.approvalGate ? (
                    <div className="wfd-approval-banner">
                      <strong>Approval gate · {step.approvalGate.channel}</strong>
                      <br />
                      {step.approvalGate.reason}
                    </div>
                  ) : null}
                </div>
              </div>
            );
          })}
        </div>

        <div className="wfd-section-label">RUN HISTORY · LAST 7 DAYS</div>
        <div
          style={{
            padding: 20,
            background: "white",
            border: "1px solid var(--border, #ece9f5)",
            borderRadius: 12,
            display: "flex",
            gap: 24,
            flexWrap: "wrap",
            fontFamily: "Inter, sans-serif",
            fontSize: 13,
            color: "var(--muted-foreground, #52525b)",
          }}
        >
          <div>
            <div
              style={{
                fontSize: 28,
                fontWeight: 900,
                color: "var(--foreground, #1e1b4b)",
                lineHeight: 1,
              }}
            >
              82
            </div>
            <div
              style={{
                fontFamily: "JetBrains Mono, monospace",
                fontSize: 10,
                letterSpacing: "0.14em",
                textTransform: "uppercase",
                marginTop: 4,
              }}
            >
              Runs
            </div>
          </div>
          <div>
            <div
              style={{
                fontSize: 28,
                fontWeight: 900,
                color: "#14b8a6",
                lineHeight: 1,
              }}
            >
              94%
            </div>
            <div
              style={{
                fontFamily: "JetBrains Mono, monospace",
                fontSize: 10,
                letterSpacing: "0.14em",
                textTransform: "uppercase",
                marginTop: 4,
              }}
            >
              Success rate
            </div>
          </div>
          <div>
            <div
              style={{
                fontSize: 28,
                fontWeight: 900,
                color: "var(--foreground, #1e1b4b)",
                lineHeight: 1,
              }}
            >
              6.2s
            </div>
            <div
              style={{
                fontFamily: "JetBrains Mono, monospace",
                fontSize: 10,
                letterSpacing: "0.14em",
                textTransform: "uppercase",
                marginTop: 4,
              }}
            >
              Median latency
            </div>
          </div>
          <div>
            <div
              style={{
                fontSize: 28,
                fontWeight: 900,
                color: "var(--foreground, #1e1b4b)",
                lineHeight: 1,
              }}
            >
              $0.012
            </div>
            <div
              style={{
                fontFamily: "JetBrains Mono, monospace",
                fontSize: 10,
                letterSpacing: "0.14em",
                textTransform: "uppercase",
                marginTop: 4,
              }}
            >
              Cost / run
            </div>
          </div>
        </div>
      </div>
      <WorkflowProposeModal
        open={editChatOpen}
        onOpenChange={setEditChatOpen}
        editingWorkflow={{
          workflowName: workflow.name,
          verticalId: workflow.vertical,
        }}
      />
    </>
  );
}
