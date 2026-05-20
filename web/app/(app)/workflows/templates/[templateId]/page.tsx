"use client";

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
  data: { icon: <Database size={14} />, color: "#71717a", label: "Data" },
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
      <style jsx global>{`
        .tpldetail-stage {
          padding: 48px 32px 96px;
          max-width: 920px;
          margin: 0 auto;
        }
        .tpldetail-back {
          display: inline-flex;
          align-items: center;
          gap: 6px;
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--muted-foreground, #71717a);
          letter-spacing: 0.18em;
          text-transform: uppercase;
          font-weight: 700;
          text-decoration: none;
          margin-bottom: 24px;
        }
        .tpldetail-back:hover { color: #8b5cf6; }
        .tpldetail-head {
          padding: 28px;
          border-radius: 18px;
          background: linear-gradient(135deg, rgba(139, 92, 246, 0.06), rgba(236, 72, 153, 0.04));
          border: 1px solid var(--border, #ece9f5);
          margin-bottom: 28px;
        }
        .tpldetail-head-top {
          display: flex;
          gap: 12px;
          align-items: center;
          flex-wrap: wrap;
          margin-bottom: 14px;
        }
        .tpldetail-vertical {
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: #8b5cf6;
          letter-spacing: 0.14em;
          text-transform: uppercase;
          font-weight: 700;
          padding: 5px 12px;
          border-radius: 999px;
          background: white;
          border: 1px solid rgba(139, 92, 246, 0.2);
        }
        .tpldetail-risk {
          font-family: "JetBrains Mono", monospace;
          font-size: 10px;
          letter-spacing: 0.12em;
          text-transform: uppercase;
          font-weight: 700;
          padding: 5px 12px;
          border-radius: 999px;
          color: white;
        }
        .tpldetail-steps-pill {
          font-family: "JetBrains Mono", monospace;
          font-size: 10px;
          color: var(--muted-foreground, #71717a);
          letter-spacing: 0.12em;
          text-transform: uppercase;
          font-weight: 700;
        }
        .tpldetail-title {
          font-family: "Audiowide", "Inter", sans-serif;
          font-size: 38px;
          line-height: 1.05;
          font-weight: 400;
          margin: 6px 0 14px;
          color: var(--foreground, #1e1b4b);
        }
        .tpldetail-desc {
          font-size: 16px;
          color: var(--muted-foreground, #3f3f46);
          line-height: 1.55;
          margin: 0 0 22px;
          max-width: 680px;
        }
        .tpldetail-cta-row {
          display: flex;
          gap: 12px;
          flex-wrap: wrap;
        }
        .tpldetail-cta-primary {
          display: inline-flex;
          align-items: center;
          gap: 8px;
          padding: 12px 22px;
          border-radius: 11px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
          font-family: "Inter", sans-serif;
          font-size: 14.5px;
          font-weight: 700;
          border: none;
          cursor: pointer;
          text-decoration: none;
          box-shadow: 0 8px 18px -6px rgba(139, 92, 246, 0.45);
        }
        .tpldetail-cta-secondary {
          display: inline-flex;
          align-items: center;
          gap: 6px;
          padding: 11px 18px;
          border-radius: 11px;
          background: white;
          color: var(--foreground, #1e1b4b);
          font-family: "Inter", sans-serif;
          font-size: 14px;
          font-weight: 600;
          border: 1px solid var(--border, #ece9f5);
          text-decoration: none;
        }
        .tpldetail-section-label {
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--muted-foreground, #71717a);
          letter-spacing: 0.18em;
          text-transform: uppercase;
          font-weight: 700;
          margin: 28px 0 14px;
        }
        .tpldetail-step-list {
          display: flex;
          flex-direction: column;
          gap: 14px;
        }
        .tpldetail-step {
          display: flex;
          gap: 16px;
          padding: 18px 20px;
          background: white;
          border: 1px solid var(--border, #ece9f5);
          border-radius: 12px;
        }
        .tpldetail-step-index {
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--muted-foreground, #71717a);
          letter-spacing: 0.08em;
          font-weight: 700;
          padding-top: 4px;
          min-width: 24px;
        }
        .tpldetail-step-icon {
          width: 32px;
          height: 32px;
          border-radius: 8px;
          display: grid;
          place-items: center;
          color: white;
          flex-shrink: 0;
        }
        .tpldetail-step-body {
          flex: 1;
          min-width: 0;
        }
        .tpldetail-step-kind {
          font-family: "JetBrains Mono", monospace;
          font-size: 10px;
          letter-spacing: 0.14em;
          text-transform: uppercase;
          font-weight: 700;
          margin-bottom: 4px;
        }
        .tpldetail-step-name {
          font-family: "Inter", sans-serif;
          font-size: 16px;
          font-weight: 600;
          color: var(--foreground, #1e1b4b);
          margin: 0 0 4px;
        }
        .tpldetail-step-desc {
          font-size: 14px;
          color: var(--muted-foreground, #3f3f46);
          line-height: 1.5;
          margin: 0;
        }
        .tpldetail-step-meta {
          margin-top: 10px;
          display: flex;
          flex-wrap: wrap;
          gap: 8px;
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--muted-foreground, #71717a);
          letter-spacing: 0.04em;
        }
        .tpldetail-gate {
          margin-top: 10px;
          padding: 10px 12px;
          border-radius: 8px;
          background: rgba(245, 158, 11, 0.1);
          border: 1px solid rgba(245, 158, 11, 0.3);
          color: #92400e;
          font-size: 12.5px;
          line-height: 1.45;
        }
        .tpldetail-gate strong {
          font-family: "JetBrains Mono", monospace;
          font-size: 10px;
          letter-spacing: 0.14em;
          display: block;
          margin-bottom: 4px;
        }
      `}</style>
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
        <h3 className="tpldetail-step-name">{step.name}</h3>
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
