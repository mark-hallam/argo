"use client";

import Link from "next/link";
import { useMemo, useState } from "react";
import { ArrowLeft, ArrowRight, Layers, Search, Sparkles } from "lucide-react";
import {
  RISK_COLOR,
  RISK_LABEL,
  TEMPLATE_VERTICALS,
  VERTICAL_LABEL,
  WORKFLOW_TEMPLATES,
  type TemplateRisk,
  type TemplateVertical,
  type WorkflowTemplate,
} from "@/lib/workflow-templates";

/**
 * Workflow template gallery — `/workflows/templates`.
 *
 * 50+ pre-built workflows the operator can clone in one click. Closes
 * the n8n template-gallery moat. Each card opens a per-template detail
 * page; the detail page exposes "Use template" which seeds the
 * existing visual builder with the template's step list.
 *
 * V1 ships a curated seed (`web/lib/workflow-templates.ts`). Real
 * binding to gateway-stored templates lands in a follow-up brief.
 */

const RISK_FILTERS: { id: "all" | TemplateRisk; label: string }[] = [
  { id: "all", label: "All risk tiers" },
  { id: "approval-required", label: "Approval required" },
  { id: "autonomous", label: "Autonomous" },
];

export default function WorkflowTemplatesPage() {
  const [query, setQuery] = useState("");
  const [vertical, setVertical] = useState<"all" | TemplateVertical>("all");
  const [risk, setRisk] = useState<"all" | TemplateRisk>("all");

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    return WORKFLOW_TEMPLATES.filter((tpl) => {
      if (vertical !== "all" && tpl.vertical !== vertical) return false;
      if (risk !== "all" && tpl.risk !== risk) return false;
      if (!q) return true;
      const haystack = [
        tpl.name,
        tpl.id,
        tpl.description,
        VERTICAL_LABEL[tpl.vertical],
        ...tpl.steps.map((s) => `${s.name} ${s.description}`),
      ]
        .join(" ")
        .toLowerCase();
      return haystack.includes(q);
    });
  }, [query, vertical, risk]);

  return (
    <>
      <link rel="preconnect" href="https://fonts.googleapis.com" />
      <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
      <link
        href="https://fonts.googleapis.com/css2?family=Audiowide&family=JetBrains+Mono:wght@400;500;600;700&display=swap"
        rel="stylesheet"
      />
      <style jsx global>{`
        .templates-stage {
          padding: 48px 32px 96px;
          max-width: 1180px;
          margin: 0 auto;
        }
        .templates-back {
          display: inline-flex;
          align-items: center;
          gap: 6px;
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--muted-foreground, #52525b);
          letter-spacing: 0.18em;
          text-transform: uppercase;
          font-weight: 700;
          text-decoration: none;
          margin-bottom: 22px;
        }
        .templates-back:hover { color: #8b5cf6; }
        .templates-eyebrow {
          display: inline-flex;
          align-items: center;
          gap: 6px;
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--muted-foreground, #52525b);
          letter-spacing: 0.2em;
          text-transform: uppercase;
          font-weight: 700;
          margin-bottom: 18px;
          padding: 5px 14px;
          border-radius: 999px;
          border: 1px solid var(--border, #ece9f5);
          background: var(--background, #fafaff);
        }
        .templates-hero {
          font-family: "Audiowide", "Inter", sans-serif;
          font-size: 48px;
          line-height: 1.05;
          font-weight: 400;
          margin: 0 0 14px;
          color: var(--foreground, #1e1b4b);
        }
        .templates-hero strong {
          font-weight: 400;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          -webkit-background-clip: text;
          background-clip: text;
          color: transparent;
        }
        .templates-sub {
          font-size: 16px;
          font-weight: 300;
          color: var(--muted-foreground, #3f3f46);
          margin: 0 0 28px;
          max-width: 660px;
        }
        .templates-search-row {
          display: flex;
          align-items: center;
          gap: 14px;
          padding: 16px 20px;
          border-radius: 14px;
          border: 1px solid var(--border, #ece9f5);
          background: white;
          margin-bottom: 18px;
          box-shadow: 0 1px 0 rgba(0, 0, 0, 0.02);
        }
        .templates-search-icon {
          width: 28px;
          height: 28px;
          border-radius: 8px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
          display: grid;
          place-items: center;
          flex-shrink: 0;
        }
        .templates-search-input {
          flex: 1;
          border: none;
          outline: none;
          font-family: "Inter", sans-serif;
          font-size: 18px;
          color: var(--foreground, #1e1b4b);
          background: transparent;
        }
        .templates-count-pill {
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--muted-foreground, #52525b);
          letter-spacing: 0.08em;
        }
        .templates-filter-bar {
          display: flex;
          flex-wrap: wrap;
          gap: 8px;
          margin-bottom: 16px;
          align-items: center;
        }
        .templates-filter-label {
          font-family: "JetBrains Mono", monospace;
          font-size: 10px;
          color: var(--muted-foreground, #52525b);
          letter-spacing: 0.18em;
          text-transform: uppercase;
          font-weight: 700;
          margin-right: 6px;
        }
        .templates-chip {
          padding: 6px 12px;
          border-radius: 999px;
          background: white;
          border: 1px solid var(--border, #ece9f5);
          color: var(--foreground, #1e1b4b);
          font-size: 13px;
          cursor: pointer;
          font-family: "Inter", sans-serif;
        }
        .templates-chip.is-active {
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
          border-color: transparent;
          box-shadow: 0 4px 10px -3px rgba(139, 92, 246, 0.45);
        }
        .templates-section-label {
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--muted-foreground, #52525b);
          letter-spacing: 0.16em;
          text-transform: uppercase;
          font-weight: 700;
          margin: 36px 0 14px;
          display: flex;
          align-items: center;
          justify-content: space-between;
        }
        .templates-grid {
          display: grid;
          grid-template-columns: repeat(3, 1fr);
          gap: 16px;
        }
        @media (max-width: 1024px) {
          .templates-grid { grid-template-columns: repeat(2, 1fr); }
        }
        @media (max-width: 640px) {
          .templates-grid { grid-template-columns: 1fr; }
        }
        .template-card {
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
          display: flex;
          flex-direction: column;
        }
        .template-card:hover {
          border-color: rgba(139, 92, 246, 0.4);
          transform: translateY(-2px);
          box-shadow: 0 12px 24px -10px rgba(139, 92, 246, 0.18);
        }
        .template-card-top {
          display: flex;
          justify-content: space-between;
          align-items: flex-start;
          margin-bottom: 12px;
        }
        .template-card-vertical {
          font-family: "JetBrains Mono", monospace;
          font-size: 10.5px;
          color: #8b5cf6;
          letter-spacing: 0.12em;
          text-transform: uppercase;
          font-weight: 700;
        }
        .template-card-risk {
          font-family: "JetBrains Mono", monospace;
          font-size: 9.5px;
          letter-spacing: 0.1em;
          text-transform: uppercase;
          font-weight: 700;
          padding: 3px 9px;
          border-radius: 999px;
          color: white;
        }
        .template-card-name {
          font-family: "Inter", sans-serif;
          font-weight: 600;
          font-size: 18px;
          color: var(--foreground, #1e1b4b);
          margin: 0 0 6px;
        }
        .template-card-desc {
          font-size: 13.5px;
          color: var(--muted-foreground, #3f3f46);
          line-height: 1.5;
          margin: 0 0 18px;
        }
        .template-card-step-row {
          display: flex;
          gap: 8px;
          align-items: center;
          margin-top: auto;
        }
        .template-step-pill {
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
        .template-step-dots {
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
        .template-step-count {
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--muted-foreground, #52525b);
          letter-spacing: 0.04em;
          font-weight: 600;
        }
        .template-card-arrow {
          position: absolute;
          right: 18px;
          bottom: 18px;
          opacity: 0;
          transform: translateX(-4px);
          transition: all 0.2s;
          color: #8b5cf6;
        }
        .template-card:hover .template-card-arrow {
          opacity: 1;
          transform: translateX(0);
        }
        .templates-empty {
          padding: 64px 20px;
          text-align: center;
          color: var(--muted-foreground, #52525b);
          background: white;
          border: 1px dashed var(--border, #ece9f5);
          border-radius: 14px;
        }
      `}</style>
      <div className="templates-stage">
        <Link href="/workflows" className="templates-back">
          <ArrowLeft size={11} /> ALL WORKFLOWS
        </Link>
        <div className="templates-eyebrow">
          <Layers size={11} /> TEMPLATE GALLERY · {WORKFLOW_TEMPLATES.length} READY-TO-CLONE
        </div>
        <h1 className="templates-hero">
          Don&apos;t start from
          <br />
          a <strong>blank canvas</strong>.
        </h1>
        <p className="templates-sub">
          Pre-built playbooks across sales, support, finance, people-ops, marketing,
          ops, engineering, and compliance. One click to clone into your builder.
        </p>

        <div className="templates-search-row">
          <div className="templates-search-icon">
            <Search size={16} />
          </div>
          <input
            type="text"
            className="templates-search-input"
            placeholder="Search 50+ templates by name, step, or description…"
            aria-label="Search templates"
            data-testid="templates-search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <span className="templates-count-pill">
            {filtered.length} of {WORKFLOW_TEMPLATES.length}
          </span>
        </div>

        <div className="templates-filter-bar">
          <span className="templates-filter-label">VERTICAL</span>
          <button
            type="button"
            className={`templates-chip${vertical === "all" ? " is-active" : ""}`}
            onClick={() => setVertical("all")}
            data-testid="templates-vertical-all"
          >
            All
          </button>
          {TEMPLATE_VERTICALS.map((v) => (
            <button
              key={v}
              type="button"
              className={`templates-chip${vertical === v ? " is-active" : ""}`}
              onClick={() => setVertical(v)}
              data-testid={`templates-vertical-${v}`}
            >
              {VERTICAL_LABEL[v]}
            </button>
          ))}
        </div>

        <div className="templates-filter-bar">
          <span className="templates-filter-label">RISK</span>
          {RISK_FILTERS.map((r) => (
            <button
              key={r.id}
              type="button"
              className={`templates-chip${risk === r.id ? " is-active" : ""}`}
              onClick={() => setRisk(r.id)}
              data-testid={`templates-risk-${r.id}`}
            >
              {r.label}
            </button>
          ))}
        </div>

        {filtered.length === 0 ? (
          <div className="templates-empty">
            <Sparkles size={20} style={{ marginBottom: 8 }} />
            <p style={{ margin: 0, fontFamily: "Inter, sans-serif" }}>
              No templates match those filters. Try clearing the search or pick a different vertical.
            </p>
          </div>
        ) : (
          <>
            <div className="templates-section-label">
              <span>BROWSE</span>
              <span>{filtered.length} TEMPLATES</span>
            </div>
            <div className="templates-grid" data-testid="templates-grid">
              {filtered.map((tpl) => (
                <TemplateCard key={tpl.id} template={tpl} />
              ))}
            </div>
          </>
        )}
      </div>
    </>
  );
}

function TemplateCard({ template }: { template: WorkflowTemplate }) {
  return (
    <Link
      href={`/workflows/templates/${template.id}`}
      className="template-card"
      data-testid={`template-card-${template.id}`}
    >
      <div className="template-card-top">
        <span className="template-card-vertical">{VERTICAL_LABEL[template.vertical]}</span>
        <span
          className="template-card-risk"
          style={{ background: RISK_COLOR[template.risk] }}
        >
          {RISK_LABEL[template.risk]}
        </span>
      </div>
      <h3 className="template-card-name">{template.name}</h3>
      <p className="template-card-desc">{template.description}</p>
      <div className="template-card-step-row">
        <div className="template-step-pill">{template.steps.length}</div>
        <div className="template-step-dots" />
        <span className="template-step-count">{template.steps.length} steps</span>
      </div>
      <ArrowRight size={18} className="template-card-arrow" />
    </Link>
  );
}
