"use client";

import { useMemo, useState } from "react";
import Link from "next/link";
import { GitBranch, ArrowRight, Plus, Search, FileCode2, Clock, Sparkles } from "lucide-react";

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
      <style jsx global>{`
        .workflows-stage {
          padding: 48px 32px 96px;
          max-width: 1100px;
          margin: 0 auto;
        }
        .workflows-eyebrow {
          display: inline-flex;
          align-items: center;
          gap: 6px;
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--muted-foreground, #71717a);
          letter-spacing: 0.2em;
          text-transform: uppercase;
          font-weight: 700;
          margin-bottom: 18px;
          padding: 5px 14px;
          border-radius: 999px;
          border: 1px solid var(--border, #ece9f5);
          background: var(--background, #fafaff);
        }
        .workflows-eyebrow-dot {
          width: 6px;
          height: 6px;
          border-radius: 50%;
          background: #14b8a6;
          box-shadow: 0 0 6px #14b8a6;
        }
        .workflows-hero {
          font-family: "Audiowide", "Inter", sans-serif;
          font-size: 48px;
          line-height: 1.05;
          font-weight: 400;
          letter-spacing: 0;
          margin: 0 0 14px;
          color: var(--foreground, #1e1b4b);
        }
        .workflows-hero strong {
          font-weight: 400;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          -webkit-background-clip: text;
          background-clip: text;
          color: transparent;
        }
        .workflows-sub {
          font-size: 16px;
          font-weight: 300;
          color: var(--muted-foreground, #3f3f46);
          margin: 0 0 28px;
          max-width: 620px;
        }
        .workflows-search-row {
          display: flex;
          align-items: center;
          gap: 14px;
          padding: 16px 20px;
          border-radius: 14px;
          border: 1px solid var(--border, #ece9f5);
          background: white;
          margin-bottom: 14px;
          box-shadow: 0 1px 0 rgba(0, 0, 0, 0.02);
        }
        .workflows-search-icon {
          width: 28px;
          height: 28px;
          border-radius: 8px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
          display: grid;
          place-items: center;
          flex-shrink: 0;
        }
        .workflows-search-input {
          flex: 1;
          border: none;
          outline: none;
          font-family: "Inter", sans-serif;
          font-size: 18px;
          color: var(--foreground, #1e1b4b);
          background: transparent;
        }
        .workflows-count-pill {
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--muted-foreground, #71717a);
          letter-spacing: 0.08em;
        }
        .workflows-propose {
          display: inline-flex;
          align-items: center;
          gap: 8px;
          padding: 9px 16px;
          border-radius: 10px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
          font-family: "Inter", sans-serif;
          font-size: 14px;
          font-weight: 600;
          border: none;
          cursor: pointer;
          text-decoration: none;
          box-shadow: 0 4px 12px -2px rgba(139, 92, 246, 0.4);
        }
        .workflows-templates-cta {
          display: inline-flex;
          align-items: center;
          gap: 6px;
          padding: 9px 14px;
          border-radius: 10px;
          background: white;
          color: #8b5cf6;
          font-family: "Inter", sans-serif;
          font-size: 13.5px;
          font-weight: 600;
          border: 1px solid #ece9f5;
          cursor: pointer;
          text-decoration: none;
          transition: all 0.14s;
        }
        .workflows-templates-cta:hover {
          border-color: rgba(139, 92, 246, 0.4);
          color: #ec4899;
          transform: translateY(-1px);
        }
        .workflows-try-row {
          display: flex;
          flex-wrap: wrap;
          gap: 8px;
          margin-bottom: 48px;
          font-size: 13px;
          align-items: center;
        }
        .workflows-try-label {
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--muted-foreground, #71717a);
          letter-spacing: 0.16em;
          text-transform: uppercase;
          font-weight: 700;
          padding: 6px 0;
          margin-right: 4px;
        }
        .workflows-try-chip {
          padding: 6px 12px;
          border-radius: 999px;
          background: white;
          border: 1px solid var(--border, #ece9f5);
          color: var(--foreground, #1e1b4b);
          font-size: 13px;
          cursor: pointer;
        }
        .workflows-section-label {
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--muted-foreground, #71717a);
          letter-spacing: 0.16em;
          text-transform: uppercase;
          font-weight: 700;
          margin-bottom: 14px;
          display: flex;
          align-items: center;
          justify-content: space-between;
        }
        .workflows-grid {
          display: grid;
          grid-template-columns: repeat(2, 1fr);
          gap: 16px;
        }
        @media (max-width: 760px) {
          .workflows-grid {
            grid-template-columns: 1fr;
          }
        }
        .workflow-card {
          background: white;
          border: 1px solid var(--border, #ece9f5);
          border-radius: 14px;
          padding: 20px;
          cursor: pointer;
          transition: all 0.18s;
          position: relative;
          overflow: hidden;
          color: inherit;
          text-decoration: none;
          display: block;
        }
        .workflow-card:hover {
          border-color: rgba(139, 92, 246, 0.4);
          transform: translateY(-2px);
          box-shadow: 0 12px 24px -10px rgba(139, 92, 246, 0.18);
        }
        .workflow-card-top {
          display: flex;
          justify-content: space-between;
          align-items: flex-start;
          margin-bottom: 12px;
        }
        .workflow-card-vertical {
          font-family: "JetBrains Mono", monospace;
          font-size: 10.5px;
          color: #8b5cf6;
          letter-spacing: 0.12em;
          text-transform: uppercase;
          font-weight: 700;
        }
        .workflow-card-risk {
          font-family: "JetBrains Mono", monospace;
          font-size: 9.5px;
          letter-spacing: 0.1em;
          text-transform: uppercase;
          font-weight: 700;
          padding: 3px 9px;
          border-radius: 999px;
          color: white;
        }
        .workflow-card-name {
          font-family: "Inter", sans-serif;
          font-weight: 600;
          font-size: 19px;
          color: var(--foreground, #1e1b4b);
          margin: 0 0 6px;
        }
        .workflow-card-desc {
          font-size: 13.5px;
          color: var(--muted-foreground, #3f3f46);
          line-height: 1.5;
          margin: 0 0 18px;
        }
        .workflow-card-steps {
          display: flex;
          gap: 8px;
          align-items: center;
          margin-bottom: 16px;
        }
        .workflow-step-pill {
          width: 28px;
          height: 28px;
          display: grid;
          place-items: center;
          border-radius: 7px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          font-weight: 700;
        }
        .workflow-step-dots {
          flex: 1;
          height: 2px;
          background: linear-gradient(
            90deg,
            #8b5cf6 0%,
            #ec4899 40%,
            #ff6b6b 80%,
            transparent 100%
          );
          border-radius: 1px;
        }
        .workflow-step-count {
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--muted-foreground, #71717a);
          letter-spacing: 0.04em;
          font-weight: 600;
        }
        .workflow-card-meta {
          display: flex;
          gap: 14px;
          font-family: "JetBrains Mono", monospace;
          font-size: 10.5px;
          color: var(--muted-foreground, #71717a);
          letter-spacing: 0.04em;
          text-transform: uppercase;
          font-weight: 600;
        }
        .workflow-card-meta-item {
          display: inline-flex;
          align-items: center;
          gap: 4px;
        }
        .workflow-card-arrow {
          position: absolute;
          right: 18px;
          bottom: 18px;
          opacity: 0;
          transform: translateX(-4px);
          transition: all 0.2s;
          color: #8b5cf6;
        }
        .workflow-card:hover .workflow-card-arrow {
          opacity: 1;
          transform: translateX(0);
        }
      `}</style>
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
          <Link
            href="/workflows/templates"
            className="workflows-propose"
            data-testid="workflows-propose"
            title="Start a new workflow from one of 50+ templates"
          >
            <Plus size={14} />
            Propose new
          </Link>
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
              color: "var(--muted-foreground, #71717a)",
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
                  style={{ background: RISK_COLOR[wf.risk] ?? "#71717a" }}
                >
                  {RISK_LABEL[wf.risk] ?? wf.risk}
                </span>
              </div>
              <h3 className="workflow-card-name">{wf.name}</h3>
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
    </>
  );
}
