import type { WorkflowSpec } from "./types";

export const SEED_WORKFLOWS: Record<string, WorkflowSpec> = {
  "gtm-engineer/post_listing": {
    name: "post_listing",
    vertical: "gtm-engineer",
    description: "Validate inventory, generate copy, post to Facebook Marketplace after seller approval.",
    version: "v3",
    risk: "approval-required",
    steps: [
      { id: "s1", kind: "trigger", triggerKind: "form", name: "Seller submits inventory", description: "Slack/email/web inbox with photos + price + condition.", inputs: ["photos[]", "price", "condition"] },
      { id: "s2", kind: "skill_invoke", name: "Validate inventory", description: "Checks price vs market, runs OCR for category hints.", outputs: ["validation_report"] },
      {
        id: "s3",
        kind: "decision",
        name: "Branch on validation",
        description: "Below floor → manager review. Otherwise continue.",
        branch: {
          trueLabel: "passes",
          falseLabel: "below floor",
          joinAt: "s4",
          falseSteps: [
            { id: "s3b1", kind: "approval", name: "Manager reviews exception", description: "Floor-price exception requires manager OK before listing copy is drafted.", approvalGate: { reason: "Pricing exceptions need manager signoff.", channel: "Slack" } },
          ],
        },
      },
      { id: "s4", kind: "skill_invoke", name: "Generate listing copy", description: "60-word description + 3 highlights + photos.", outputs: ["listing.title", "listing.body"] },
      { id: "s5", kind: "approval", name: "Seller approves", description: "One-tap approve / request edits via Slack.", approvalGate: { reason: "Posting public listings needs seller signoff.", channel: "Slack" } },
      { id: "s6", kind: "tool_call", name: "Post to Marketplace", description: "BrowserExecutor opens FB, fills form, submits.", outputs: ["listing.url"] },
    ],
  },
  "sales/qualify_lead": {
    name: "qualify_lead",
    vertical: "sales",
    description: "BANT-style qualification + calendar-link insertion + CRM update on accept.",
    version: "v4",
    risk: "approval-required",
    steps: [
      { id: "s1", kind: "trigger", triggerKind: "webhook", name: "Lead from web / paid / referral", description: "HubSpot/Salesforce webhook fires." },
      { id: "s2", kind: "skill_invoke", name: "Enrich + BANT score", description: "Firmographic + technographic data; 0-100 BANT score." },
      {
        id: "s3",
        kind: "decision",
        name: "Branch on BANT score",
        description: "≥75 = demo. 50-74 = nurture. <50 = decline.",
        branch: {
          trueLabel: "≥75 demo-ready",
          falseLabel: "<75 nurture",
          falseSteps: [
            { id: "s3b1", kind: "skill_invoke", name: "Drip nurture sequence", description: "Add lead to 8-week BANT nurture cadence; re-score on engagement.", outputs: ["nurture.cadence_id"] },
          ],
        },
      },
      { id: "s4", kind: "approval", name: "AE approves outreach", description: "Slack DM with qual summary + one-tap approve.", approvalGate: { reason: "Outreach to enterprise needs AE signoff.", channel: "Slack" } },
      { id: "s5", kind: "tool_call", name: "Send outreach + calendar link", description: "Gmail/Outlook + Calendly slot + CRM update." },
    ],
  },
  "gtm-engineer/full_listing_flow": {
    name: "full_listing_flow",
    vertical: "gtm-engineer",
    description: "End-to-end listing pipeline composed from existing sub-workflows.",
    version: "v1",
    risk: "approval-required",
    steps: [
      { id: "s1", kind: "trigger", triggerKind: "form", name: "Seller submits inventory", description: "Same intake form as post_listing." },
      {
        id: "s2",
        kind: "sub_workflow",
        name: "Call post_listing",
        description: "Invokes gtm-engineer/post_listing (6 steps).",
        subWorkflow: {
          verticalId: "gtm-engineer",
          name: "post_listing",
          displayName: "post listing",
          stepCount: 6,
        },
      },
      {
        id: "s3",
        kind: "sub_workflow",
        name: "Call qualify_lead",
        description: "Invokes sales/qualify_lead (5 steps) on the seller as a follow-up lead.",
        subWorkflow: {
          verticalId: "sales",
          name: "qualify_lead",
          displayName: "qualify lead",
          stepCount: 5,
        },
      },
      { id: "s4", kind: "notify", name: "Notify ops", description: "Post the resulting listing URL + lead status to #ops." },
    ],
  },
  "support/qa_audit": {
    name: "qa_audit",
    vertical: "support",
    description: "Daily QA pass over yesterday's tickets — score each, wait between provider calls, summarise to Slack.",
    version: "v1",
    risk: "autonomous",
    steps: [
      { id: "s1", kind: "trigger", name: "Nightly cron at 02:00 UTC", description: "Daily quality-audit run kicks off after support shift ends.", outputs: ["window.start", "window.end"] },
      { id: "s2", kind: "skill_invoke", name: "Fetch yesterday's tickets", description: "Pulls closed tickets from Zendesk for the audit window.", outputs: ["tickets[]"] },
      {
        id: "s3",
        kind: "loop",
        name: "For each ticket",
        description: "Iterate audit work over every ticket in the window.",
        loopConfig: { items_path: "$.tickets", item_var_name: "ticket", max_iterations: 500 },
        body: [
          { id: "s3a", kind: "skill_invoke", name: "Score ticket quality", description: "Tone, resolution, escalation timing, customer effort." },
          { id: "s3b", kind: "decision", name: "Below quality floor?", description: "Score < 60 → flag for coaching review." },
          { id: "s3c", kind: "wait", name: "Throttle provider", description: "Avoid rate-limit on the LLM.", waitConfig: { duration_seconds: 2 } },
        ],
      },
      { id: "s4", kind: "wait", name: "Hold for shift handover", description: "Defer summary until the next on-duty manager is online.", waitConfig: { until_field: "$.next_shift_start" } },
      { id: "s5", kind: "notify", name: "Post summary to Slack", description: "QA scorecard + flagged tickets posted to #support-coaching." },
    ],
  },
};
