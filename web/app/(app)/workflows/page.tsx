"use client";
import "./page.css";

import { useMemo, useState } from "react";
import Link from "next/link";
import { GitBranch, ArrowRight, Plus, Search, FileCode2, Clock, Sparkles } from "lucide-react";
import { WorkflowProposeModal } from "@/components/skycot/WorkflowProposeModal";

/**
 * Workflows index — `/workflows` brand-loud catalog view.
 *
 * Surfaces every workflow definition across the tenant's verticals
 * in one searchable grid. Each workflow card links to the existing
 * version viewer at `/verticals/<v>/workflows/<n>`, which already
 * exposes side-by-side diff between versions.
 *
 * V1 ships a curated seed of demo workflows so the surface has body
 * for the operator demo recording. Real-time data binding (live
 * fetch of all workflows across all verticals) lands in a follow-up
 * brief — the gateway has the routes (`POST/GET/PUT/DELETE
 * /api/v1/verticals/<v>/workflows[...]`) from PR #1860.
 */

const SEED_WORKFLOWS = [
  {
    name: "post_listing",
    vertical: "gtm-engineer",
    description: "Validate inventory data + post fresh listing to Facebook Marketplace.",
    steps: 6,
    versions: 3,
    lastEdited: "2 days ago",
    lastEditor: "Mariah",
    risk: "approval-required",
  },
  {
    name: "screen_candidate",
    vertical: "people-ops",
    description: "First-pass candidate screen against a JD with skill-match scoring.",
    steps: 8,
    versions: 5,
    lastEdited: "4 hours ago",
    lastEditor: "Priya",
    risk: "autonomous",
  },
  {
    name: "approve_refund",
    vertical: "support",
    description: "Verify refund eligibility, check ledger, route high-value to manager.",
    steps: 5,
    versions: 2,
    lastEdited: "1 week ago",
    lastEditor: "Jamal",
    risk: "approval-required",
  },
  {
    name: "extract_invoice",
    vertical: "finance",
    description: "Pull line items from PDF/scanned invoices, reconcile vendor.",
    steps: 4,
    versions: 1,
    lastEdited: "12 hours ago",
    lastEditor: "Sam",
    risk: "autonomous",
  },
  {
    name: "qualify_lead",
    vertical: "sales",
    description: "Budget / Authority / Need / Timeline qualification + calendar-link insertion + CRM update.",
    steps: 7,
    versions: 4,
    lastEdited: "3 days ago",
    lastEditor: "Rio",
    risk: "approval-required",
  },
  {
    name: "triage_ticket",
    vertical: "support",
    description: "Classify tickets, deflect with KB articles, escalate the rest.",
    steps: 3,
    versions: 6,
    lastEdited: "32 minutes ago",
    lastEditor: "Jamal",
    risk: "autonomous",
  },
  {
    name: "exit_interview_synth",
    vertical: "people-ops",
    description: "Quarterly synthesis of exit-interview transcripts into a retention report.",
    steps: 3,
    versions: 2,
    lastEdited: "yesterday",
    lastEditor: "Priya",
    risk: "approval-required",
  },
  {
    name: "draft_status_report",
    vertical: "marketing",
    description: "Weekly client status report — campaign perf + content shipped + next-week asks.",
    steps: 4,
    versions: 3,
    lastEdited: "12 minutes ago",
    lastEditor: "Nori",
    risk: "approval-required",
  },
];

const TRY_CHIPS = ["approval flows", "data extraction", "outbound qualification", "escalation"];

const RISK_LABEL: Record<string, string> = {
  "approval-required": "Approval required",
  autonomous: "Autonomous",
};

const RISK_COLOR: Record<string, string> = {
  "approval-required": "#f59e0b",
  autonomous: "#14b8a6",
};

export default function WorkflowsPage() {
  const [query, setQuery] = useState("");
  const [proposeOpen, setProposeOpen] = useState(false);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return SEED_WORKFLOWS;
    return SEED_WORKFLOWS.filter(
      (wf) =>
        wf.name.toLowerCase().includes(q) ||
        wf.vertical.toLowerCase().includes(q) ||
        wf.description.toLowerCase().includes(q),
    );
  }, [query]);

  return (
    <>
      <link rel="preconnect" href="https://fonts.googleapis.com" />
      <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
      <link
        href="https://fonts.googleapis.com/css2?family=Audiowide&family=JetBrains+Mono:wght@400;500;600;700&display=swap"
        rel="stylesheet"
      />
      <div className="workflows-stage">
        <div className="workflows-eyebrow">
          <span className="workflows-eyebrow-dot" />
          WORKFLOWS · LIVE
        </div>

        <h1 className="workflows-hero">
          How your teammates
          <br />
          actually <strong>think</strong>.
        </h1>

        <p className="workflows-sub">
          Workflows are the plain-English step-by-step playbooks your AI teammates follow.
          Edit them in chat, version them like code, ship to production in one click.
        </p>

        <div className="workflows-search-row">
          <div className="workflows-search-icon">
            <Search size={16} />
          </div>
          <input
            type="text"
            className="workflows-search-input"
            placeholder="Search workflows by name, vertical, or step…"
            aria-label="Search workflows"
            data-testid="workflows-search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <span className="workflows-count-pill">
            {filtered.length === SEED_WORKFLOWS.length
              ? `${SEED_WORKFLOWS.length} workflows`
              : `${filtered.length} of ${SEED_WORKFLOWS.length}`}
          </span>
          <Link
            href="/workflows/templates"
            className="workflows-templates-cta"
            data-testid="workflows-templates-cta"
          >
            <Sparkles size={13} />
            Browse 50+ templates
          </Link>
          <button
            type="button"
            className="workflows-propose"
            data-testid="workflows-propose"
            title="Describe a new workflow in plain English"
            onClick={() => setProposeOpen(true)}
          >
            <Plus size={14} />
            Propose new
          </button>
        </div>

        <div className="workflows-try-row">
          <span className="workflows-try-label">TRY</span>
          {TRY_CHIPS.map((chip) => (
            <button
              key={chip}
              type="button"
              className="workflows-try-chip"
              onClick={() => setQuery(chip)}
            >
              {chip}
            </button>
          ))}
        </div>

        <div className="workflows-section-label">
          <span>{query.trim() ? `MATCHING "${query.trim()}"` : "ALL WORKFLOWS"}</span>
          <span>NEWEST FIRST</span>
        </div>

        {filtered.length === 0 ? (
          <div
            data-testid="workflows-empty"
            style={{
              gridColumn: "1 / -1",
              padding: "48px 24px",
              textAlign: "center",
              border: "1px dashed var(--border, #ece9f5)",
              borderRadius: "16px",
              color: "var(--muted-foreground, #52525b)",
            }}
          >
            <p style={{ fontSize: 16, fontWeight: 600, color: "var(--foreground, #1e1b4b)", margin: "0 0 6px" }}>
              No workflows match &ldquo;{query.trim()}&rdquo;.
            </p>
            <p style={{ fontSize: 14, margin: "0 0 16px" }}>
              Try a different query or browse the 50+ templates.
            </p>
            <div style={{ display: "inline-flex", gap: 12 }}>
              <button
                type="button"
                onClick={() => setQuery("")}
                style={{
                  padding: "8px 16px",
                  borderRadius: 8,
                  border: "1px solid var(--border, #ece9f5)",
                  background: "white",
                  cursor: "pointer",
                  fontSize: 13,
                  fontWeight: 600,
                  color: "var(--foreground, #1e1b4b)",
                }}
              >
                Clear search
              </button>
              <Link
                href="/workflows/templates"
                style={{
                  padding: "8px 16px",
                  borderRadius: 8,
                  background: "linear-gradient(135deg, #8b5cf6, #ec4899)",
                  color: "white",
                  textDecoration: "none",
                  fontSize: 13,
                  fontWeight: 600,
                }}
              >
                Browse templates →
              </Link>
            </div>
          </div>
        ) : null}

        <div className="workflows-grid">
          {filtered.map((wf) => (
            <Link
              key={`${wf.vertical}/${wf.name}`}
              href={`/workflows/${wf.vertical}/${wf.name}`}
              className="workflow-card"
            >
              <div className="workflow-card-top">
                <span className="workflow-card-vertical">{wf.vertical}</span>
                <span
                  className="workflow-card-risk"
                  style={{ background: RISK_COLOR[wf.risk] ?? "#52525b" }}
                >
                  {RISK_LABEL[wf.risk] ?? wf.risk}
                </span>
              </div>
              <h2 className="workflow-card-name">{wf.name}</h2>
              <p className="workflow-card-desc">{wf.description}</p>

              <div className="workflow-card-steps">
                <div className="workflow-step-pill">{wf.steps}</div>
                <div className="workflow-step-dots" />
                <span className="workflow-step-count">{wf.steps} steps</span>
              </div>

              <div className="workflow-card-meta">
                <span className="workflow-card-meta-item">
                  <GitBranch size={11} /> v{wf.versions}
                </span>
                <span className="workflow-card-meta-item">
                  <Clock size={11} /> {wf.lastEdited}
                </span>
                <span className="workflow-card-meta-item">by {wf.lastEditor}</span>
              </div>

              <ArrowRight size={18} className="workflow-card-arrow" />
            </Link>
          ))}
        </div>
      </div>
      <WorkflowProposeModal open={proposeOpen} onOpenChange={setProposeOpen} />
    </>
  );
}
