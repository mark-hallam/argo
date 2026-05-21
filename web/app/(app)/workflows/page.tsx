"use client";
import "./page.css";

import { useMemo, useState } from "react";
import Link from "next/link";
import { GitBranch, ArrowRight, Plus, Search, Clock, Sparkles } from "lucide-react";
import { WorkflowProposeModal } from "@/components/skycot/WorkflowProposeModal";
import { HoverHint } from "@/components/ui/Tooltip";
import { useWorkflows } from "@/lib/hooks/use-workflows";
import { SAMPLE_WORKFLOW_SUMMARIES } from "@/lib/sample-workflows";

/**
 * Workflows index — `/workflows` brand-loud catalog view.
 *
 * Lives off `useWorkflows()` which fans the per-vertical gateway call
 * out in parallel (`GET /api/v1/verticals/<v>/workflows`). When the
 * gateway has no live data (offline dev, cold store, demo recording)
 * the hook falls through to the local seed in
 * `lib/sample-workflows` and surfaces `dataSource: "mock"` so the
 * hero pill flips LIVE → MOCK.
 *
 * The per-card surface is unchanged from the seed-only version — the
 * data binding is the only swap; visual / interaction tests against
 * the catalog continue to pass.
 */

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
  const { data, isLoading, isError } = useWorkflows();

  const workflows = data?.workflows ?? SAMPLE_WORKFLOW_SUMMARIES;
  const source = data?.source ?? "mock-offline";
  const isLive = source === "gateway";

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return workflows;
    return workflows.filter(
      (wf) =>
        wf.name.toLowerCase().includes(q) ||
        wf.vertical.toLowerCase().includes(q) ||
        wf.description.toLowerCase().includes(q),
    );
  }, [query, workflows]);

  return (
    <>
      <div className="workflows-stage">
        <div
          className="workflows-eyebrow"
          data-testid="workflows-source-pill"
          data-source={source}
        >
          <span className="workflows-eyebrow-dot" />
          WORKFLOWS · {isLive ? "LIVE" : "MOCK"}
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
            {filtered.length === workflows.length
              ? `${workflows.length} workflows`
              : `${filtered.length} of ${workflows.length}`}
          </span>
          <Link
            href="/workflows/templates"
            className="workflows-templates-cta"
            data-testid="workflows-templates-cta"
          >
            <Sparkles size={13} />
            Browse 50+ templates
          </Link>
          <HoverHint label="Describe a new workflow in plain English">
            <button
              type="button"
              className="workflows-propose"
              data-testid="workflows-propose"
              onClick={() => setProposeOpen(true)}
            >
              <Plus size={14} />
              Propose new
            </button>
          </HoverHint>
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

        {isLoading ? (
          <div
            data-testid="workflows-loading"
            style={{
              padding: "48px 24px",
              textAlign: "center",
              color: "var(--muted-foreground, #52525b)",
              fontSize: 14,
            }}
          >
            Loading workflows…
          </div>
        ) : null}

        {isError ? (
          <div
            data-testid="workflows-error"
            style={{
              padding: "32px 24px",
              textAlign: "center",
              color: "#dc2626",
              border: "1px solid #fecaca",
              borderRadius: 12,
              background: "#fef2f2",
              fontSize: 14,
            }}
          >
            Couldn&apos;t reach the gateway. Showing seed data so the demo still works.
          </div>
        ) : null}

        {!isLoading && filtered.length === 0 ? (
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
            <p
              style={{
                fontSize: 16,
                fontWeight: 600,
                color: "var(--foreground, #1e1b4b)",
                margin: "0 0 6px",
              }}
            >
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
