"use client";
import "./page.css";

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
      <h2 className="template-card-name">{template.name}</h2>
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
