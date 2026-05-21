"use client";

import { useCallback, useState } from "react";
import {
  ArrowLeft,
  Check,
  Clipboard,
  Clock,
  Database,
  GitBranch,
  Laptop,
  Layers,
  MessageSquare,
  Play,
  Repeat,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import Link from "next/link";
import type { ReactNode } from "react";

import type {
  StepKind,
  WorkflowSpec,
  WorkflowStep,
} from "@/app/(app)/workflows/[verticalId]/[name]/build/types";

/**
 * Mobile gate for `/workflows/<v>/<n>/build`.
 *
 * The visual workflow builder (React Flow canvas + 360px-fixed side
 * panel) is unusable below 1024px. Instead of showing a broken layout,
 * this gate renders the workflow's step list read-only and points the
 * operator at a desktop session — with a one-tap "Copy link" button so
 * they can paste the URL into their laptop browser.
 *
 * Toggled by CSS `@media (max-width: 1023px)` declared in
 * `build/page.css`; survives SSR (no JS viewport detection). The
 * canvas still mounts at desktop sizes; this gate is purely additive
 * markup with `display:none` by default.
 */

interface MobileBuilderGateProps {
  workflow: WorkflowSpec;
}

const KIND_META: Record<StepKind, { icon: ReactNode; color: string; label: string }> = {
  trigger:      { icon: <Play size={12} />,          color: "#10b981", label: "Trigger" },
  skill_invoke: { icon: <Sparkles size={12} />,      color: "#8b5cf6", label: "Skill" },
  approval:     { icon: <ShieldCheck size={12} />,   color: "#f59e0b", label: "Approval gate" },
  tool_call:    { icon: <Database size={12} />,      color: "#0891b2", label: "Tool call" },
  notify:       { icon: <MessageSquare size={12} />, color: "#ec4899", label: "Notify" },
  decision:     { icon: <GitBranch size={12} />,     color: "#6366f1", label: "Decision" },
  data:         { icon: <Database size={12} />,      color: "#52525b", label: "Data" },
  loop:         { icon: <Repeat size={12} />,        color: "#6366f1", label: "Loop" },
  wait:         { icon: <Clock size={12} />,         color: "#6366f1", label: "Wait" },
  sub_workflow: { icon: <Layers size={12} />,        color: "#0891b2", label: "Sub-workflow" },
};

function StepCard({ step, nested = false }: { step: WorkflowStep; nested?: boolean }) {
  const meta = KIND_META[step.kind];
  return (
    <li
      data-testid={`mobile-gate-step-${step.id}`}
      style={{
        listStyle: "none",
        background: "white",
        border: "1px solid #ece9f5",
        borderRadius: 12,
        padding: "14px 16px",
        boxShadow: "0 2px 8px -4px rgba(30,27,75,0.08)",
        marginLeft: nested ? 16 : 0,
        borderLeft: nested ? `3px solid ${meta.color}` : "1px solid #ece9f5",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
        <span
          aria-hidden
          style={{
            width: 26,
            height: 26,
            borderRadius: 7,
            background: meta.color,
            color: "white",
            display: "grid",
            placeItems: "center",
            flexShrink: 0,
          }}
        >
          {meta.icon}
        </span>
        <span
          style={{
            fontFamily: "JetBrains Mono, monospace",
            fontSize: 10,
            letterSpacing: "0.12em",
            textTransform: "uppercase",
            fontWeight: 700,
            color: meta.color,
          }}
        >
          {meta.label}
        </span>
      </div>
      <h3
        style={{
          margin: "8px 0 4px",
          fontSize: 15,
          fontWeight: 600,
          color: "#1e1b4b",
          lineHeight: 1.3,
        }}
      >
        {step.name}
      </h3>
      <p style={{ margin: 0, fontSize: 13, color: "#52525b", lineHeight: 1.5 }}>
        {step.description}
      </p>
      {step.approvalGate ? (
        <div
          style={{
            marginTop: 10,
            padding: "6px 10px",
            borderRadius: 7,
            background: "rgba(245,158,11,0.1)",
            border: "1px solid rgba(245,158,11,0.3)",
            color: "#92400e",
            fontSize: 11,
            fontFamily: "JetBrains Mono, monospace",
            letterSpacing: "0.08em",
            textTransform: "uppercase",
            fontWeight: 700,
          }}
        >
          Gate · {step.approvalGate.channel}
        </div>
      ) : null}
      {step.subWorkflow ? (
        <div
          data-testid={`mobile-gate-step-${step.id}-subworkflow`}
          style={{
            marginTop: 10,
            padding: "6px 10px",
            borderRadius: 7,
            background: "rgba(8,145,178,0.08)",
            border: "1px solid rgba(8,145,178,0.25)",
            color: "#0e7490",
            fontSize: 11,
            fontFamily: "JetBrains Mono, monospace",
            letterSpacing: "0.08em",
            textTransform: "uppercase",
            fontWeight: 700,
          }}
        >
          Calls {step.subWorkflow.displayName} · {step.subWorkflow.stepCount} step
          {step.subWorkflow.stepCount === 1 ? "" : "s"}
        </div>
      ) : null}
      {step.body && step.body.length > 0 ? (
        <ul
          data-testid={`mobile-gate-step-${step.id}-body`}
          style={{ margin: "12px 0 0", padding: 0, display: "flex", flexDirection: "column", gap: 10 }}
        >
          {step.body.map((child) => (
            <StepCard key={child.id} step={child} nested />
          ))}
        </ul>
      ) : null}
      {step.branch && step.branch.falseSteps.length > 0 ? (
        <div
          data-testid={`mobile-gate-step-${step.id}-branch`}
          style={{ marginTop: 12 }}
        >
          <div
            style={{
              fontFamily: "JetBrains Mono, monospace",
              fontSize: 10,
              letterSpacing: "0.12em",
              textTransform: "uppercase",
              fontWeight: 700,
              color: "#b45309",
              marginBottom: 8,
            }}
          >
            ↳ {step.branch.falseLabel} path
          </div>
          <ul style={{ margin: 0, padding: 0, display: "flex", flexDirection: "column", gap: 10 }}>
            {step.branch.falseSteps.map((child) => (
              <StepCard key={child.id} step={child} nested />
            ))}
          </ul>
        </div>
      ) : null}
    </li>
  );
}

export function MobileBuilderGate({ workflow }: MobileBuilderGateProps) {
  const [copied, setCopied] = useState(false);

  const handleCopyLink = useCallback(async () => {
    if (typeof window === "undefined") return;
    const url = window.location.href;
    try {
      if (navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(url);
      }
      setCopied(true);
      window.setTimeout(() => setCopied(false), 2000);
    } catch {
      setCopied(false);
    }
  }, []);

  return (
    <section
      data-testid="mobile-builder-gate"
      className="mobile-gate"
      aria-label="Workflow preview — open on desktop to edit"
      style={{
        minHeight: "100vh",
        background: "#fafaff",
        padding: "20px 18px 40px",
        fontFamily: "Inter, sans-serif",
      }}
    >
      <Link
        href="/workflows"
        style={{
          display: "inline-flex",
          alignItems: "center",
          gap: 6,
          fontFamily: "JetBrains Mono, monospace",
          fontSize: 10.5,
          color: "#52525b",
          letterSpacing: "0.12em",
          textTransform: "uppercase",
          fontWeight: 700,
          textDecoration: "none",
          marginBottom: 16,
        }}
      >
        <ArrowLeft size={12} /> All workflows
      </Link>

      <div
        style={{
          padding: "22px 20px",
          borderRadius: 16,
          background: "linear-gradient(135deg, #8b5cf6, #ec4899)",
          color: "white",
          boxShadow: "0 14px 30px -10px rgba(139, 92, 246, 0.45)",
          marginBottom: 22,
        }}
      >
        <div
          style={{
            display: "inline-flex",
            alignItems: "center",
            gap: 6,
            fontFamily: "JetBrains Mono, monospace",
            fontSize: 10,
            letterSpacing: "0.16em",
            textTransform: "uppercase",
            fontWeight: 700,
            opacity: 0.85,
            marginBottom: 10,
          }}
        >
          <Laptop size={12} aria-hidden /> Desktop-only editor
        </div>
        <h1
          style={{
            margin: "0 0 8px",
            fontSize: 22,
            fontWeight: 700,
            lineHeight: 1.2,
          }}
        >
          Open this on desktop to edit
        </h1>
        <p style={{ margin: "0 0 16px", fontSize: 14, lineHeight: 1.5, opacity: 0.92 }}>
          The visual canvas needs a wider screen. Here&apos;s a read-only preview of{" "}
          <strong>{workflow.name}</strong> — copy the link and open it on your laptop to make changes.
        </p>
        <button
          type="button"
          data-testid="mobile-gate-copy-link"
          onClick={handleCopyLink}
          aria-live="polite"
          style={{
            display: "inline-flex",
            alignItems: "center",
            gap: 8,
            padding: "10px 16px",
            borderRadius: 9,
            background: "white",
            color: "#1e1b4b",
            border: "none",
            fontSize: 13,
            fontWeight: 700,
            cursor: "pointer",
          }}
        >
          {copied ? (
            <>
              <Check size={14} aria-hidden /> Link copied
            </>
          ) : (
            <>
              <Clipboard size={14} aria-hidden /> Copy link
            </>
          )}
        </button>
      </div>

      <div style={{ marginBottom: 16 }}>
        <div
          style={{
            fontFamily: "JetBrains Mono, monospace",
            fontSize: 10,
            color: "#8b5cf6",
            letterSpacing: "0.14em",
            textTransform: "uppercase",
            fontWeight: 700,
            marginBottom: 4,
          }}
        >
          {workflow.vertical} · version {workflow.version}
        </div>
        <h2
          style={{
            margin: "0 0 6px",
            fontSize: 18,
            fontWeight: 700,
            color: "#1e1b4b",
          }}
        >
          {workflow.name}
        </h2>
        <p style={{ margin: 0, fontSize: 13.5, color: "#52525b", lineHeight: 1.55 }}>
          {workflow.description}
        </p>
      </div>

      <div
        style={{
          fontFamily: "JetBrains Mono, monospace",
          fontSize: 10,
          letterSpacing: "0.14em",
          textTransform: "uppercase",
          fontWeight: 700,
          color: "#52525b",
          marginBottom: 10,
        }}
      >
        {workflow.steps.length} step{workflow.steps.length === 1 ? "" : "s"}
      </div>
      <ol
        data-testid="mobile-gate-step-list"
        style={{ margin: 0, padding: 0, display: "flex", flexDirection: "column", gap: 12 }}
      >
        {workflow.steps.map((step) => (
          <StepCard key={step.id} step={step} />
        ))}
      </ol>
    </section>
  );
}
