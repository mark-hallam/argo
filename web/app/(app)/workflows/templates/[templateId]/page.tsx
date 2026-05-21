"use client";
import "./page.css";

import Link from "next/link";
import { use } from "react";
import {
  ArrowLeft,
  ArrowRight,
  Database,
  GitBranch,
  MessageSquare,
  Play,
  ShieldCheck,
  Sparkles,
  Wand2,
} from "lucide-react";
import {
  RISK_COLOR,
  RISK_LABEL,
  VERTICAL_LABEL,
  getTemplate,
  type TemplateStep,
  type TemplateStepKind,
} from "@/lib/workflow-templates";

/**
 * Per-template detail page — `/workflows/templates/[templateId]`.
 *
 * Shows the full step list before commit, with a "Use template" CTA
 * that hands off to the visual builder with the template pre-loaded.
 * The builder reads `?from_template=<id>` and pulls the steps from the
 * shared workflow-templates lib.
 */

const STEP_KIND_META: Record<
  TemplateStepKind,
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

export default function TemplateDetailPage({
  params,
}: {
  params: Promise<{ templateId: string }>;
}) {
  const { templateId } = use(params);
  const template = getTemplate(templateId);

  if (!template) {
    return (
      <div style={{ padding: 48, maxWidth: 720, margin: "0 auto", textAlign: "center" }}>
        <p style={{ color: "var(--muted-foreground)", marginBottom: 16 }}>
          Template not found.
        </p>
        <Link href="/workflows/templates" style={{ color: "#8b5cf6" }}>
          ← Browse all templates
        </Link>
      </div>
    );
  }

  const useTemplateHref = `/workflows/${template.vertical}/${template.name}/build?from_template=${template.id}`;

  return (
    <>
      <link rel="preconnect" href="https://fonts.googleapis.com" />
      <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
      <link
        href="https://fonts.googleapis.com/css2?family=Audiowide&family=JetBrains+Mono:wght@400;500;600;700&display=swap"
        rel="stylesheet"
      />
      <div className="tpldetail-stage">
        <Link href="/workflows/templates" className="tpldetail-back">
          <ArrowLeft size={11} /> ALL TEMPLATES
        </Link>

        <section className="tpldetail-head">
          <div className="tpldetail-head-top">
            <span className="tpldetail-vertical">{VERTICAL_LABEL[template.vertical]}</span>
            <span
              className="tpldetail-risk"
              style={{ background: RISK_COLOR[template.risk] }}
            >
              {RISK_LABEL[template.risk]}
            </span>
            <span className="tpldetail-steps-pill">{template.steps.length} steps</span>
          </div>
          <h1 className="tpldetail-title">{template.name}</h1>
          <p className="tpldetail-desc">{template.description}</p>
          <div className="tpldetail-cta-row">
            <Link
              href={useTemplateHref}
              className="tpldetail-cta-primary"
              data-testid="template-use-cta"
            >
              <Wand2 size={15} /> Use template
              <ArrowRight size={14} />
            </Link>
            <Link href="/workflows/templates" className="tpldetail-cta-secondary">
              Browse more templates
            </Link>
          </div>
        </section>

        <div className="tpldetail-section-label">FULL STEP LIST · WHAT THIS WORKFLOW DOES</div>
        <div className="tpldetail-step-list" data-testid="template-step-list">
          {template.steps.map((step, idx) => (
            <StepRow key={step.id} step={step} index={idx + 1} />
          ))}
        </div>
      </div>
    </>
  );
}

function StepRow({ step, index }: { step: TemplateStep; index: number }) {
  const meta = STEP_KIND_META[step.kind];
  return (
    <div className="tpldetail-step">
      <span className="tpldetail-step-index">{String(index).padStart(2, "0")}</span>
      <div className="tpldetail-step-icon" style={{ background: meta.color }}>
        {meta.icon}
      </div>
      <div className="tpldetail-step-body">
        <div className="tpldetail-step-kind" style={{ color: meta.color }}>
          {meta.label}
        </div>
        <h2 className="tpldetail-step-name">{step.name}</h2>
        <p className="tpldetail-step-desc">{step.description}</p>
        {step.inputs || step.outputs ? (
          <div className="tpldetail-step-meta">
            {step.inputs ? <span>IN: {step.inputs.join(", ")}</span> : null}
            {step.outputs ? <span>OUT: {step.outputs.join(", ")}</span> : null}
          </div>
        ) : null}
        {step.approvalGate ? (
          <div className="tpldetail-gate">
            <strong>GATE · {step.approvalGate.channel}</strong>
            {step.approvalGate.reason}
          </div>
        ) : null}
      </div>
    </div>
  );
}
