"use client";

import { Info } from "lucide-react";
import { cn } from "@/lib/utils";
import type { Node as SkillNode, NodeAnnotations } from "@/lib/types/skill-ir";
import { KIND_VISUALS } from "./NodeRenderer";

/**
 * Side panel rendered alongside the canvas. When a node is selected,
 * shows its annotations: confidence, sample count, success rate, last-
 * used timestamp, policy + tool binding. When nothing is selected, a
 * neutral hint nudges the operator toward node selection. Read-only —
 * L7's editor surfaces are deliberately absent.
 */

export interface AnnotationsPanelProps {
  node: SkillNode | null;
  className?: string;
}

export function AnnotationsPanel({ node, className }: AnnotationsPanelProps) {
  return (
    <aside
      data-testid="canvas-annotations-panel"
      aria-label="Selected node details"
      className={cn(
        "w-72 shrink-0 border-l border-[var(--border)] bg-[var(--card)] text-[var(--card-foreground)]",
        "flex flex-col overflow-y-auto",
        className,
      )}
    >
      {node ? <SelectedNodeBody node={node} /> : <EmptyHint />}
    </aside>
  );
}

function EmptyHint() {
  return (
    <div
      data-testid="canvas-annotations-empty"
      className="flex h-full flex-col items-center justify-center gap-2 px-6 py-8 text-center text-sm text-[var(--muted-foreground)]"
    >
      <Info className="size-5" aria-hidden="true" />
      <p>Select a node to inspect its annotations.</p>
    </div>
  );
}

function SelectedNodeBody({ node }: { node: SkillNode }) {
  const visual = KIND_VISUALS[node.kind.type];
  const { Icon, iconClass, label } = visual;
  const a = node.annotations;

  return (
    <div className="space-y-4 px-4 py-4">
      <header className="space-y-1">
        <div className="flex items-center gap-2 text-[10px] uppercase tracking-wide text-[var(--muted-foreground)]">
          <Icon aria-hidden="true" className={cn("size-4", iconClass)} />
          {label}
        </div>
        <h2
          data-testid="canvas-annotations-node-label"
          className="text-base font-semibold leading-tight"
        >
          {node.label}
        </h2>
        <p className="text-xs text-[var(--muted-foreground)]">id: {node.id}</p>
      </header>

      <Section title="Telemetry">
        <Row
          testId="anno-confidence"
          label="Confidence"
          value={formatPercent(a.confidence)}
          tone={confidenceTone(a.confidence)}
        />
        <Row
          testId="anno-sample-count"
          label="Samples"
          value={a.sampleCount?.toLocaleString() ?? "—"}
        />
        <Row
          testId="anno-success-rate"
          label="Success rate"
          value={formatPercent(a.successRate)}
        />
        <Row
          testId="anno-last-used"
          label="Last used"
          value={formatRelative(a.lastUsedAt)}
        />
      </Section>

      <Section title="Policy">
        <Row
          testId="anno-policy"
          label="Mode"
          value={policyLabel(a.policy)}
        />
        {a.toolBinding ? (
          <Row
            testId="anno-tool-binding"
            label="Tool"
            value={`${a.toolBinding.mcpServer} · ${a.toolBinding.toolName}`}
          />
        ) : null}
      </Section>
    </div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="space-y-1.5">
      <h3 className="text-[11px] font-medium uppercase tracking-wide text-[var(--muted-foreground)]">
        {title}
      </h3>
      <dl className="space-y-1 text-sm">{children}</dl>
    </section>
  );
}

interface RowProps {
  testId: string;
  label: string;
  value: string;
  tone?: "ok" | "warn" | "bad";
}

function Row({ testId, label, value, tone }: RowProps) {
  return (
    <div data-testid={testId} className="flex items-baseline justify-between gap-2">
      <dt className="text-xs text-[var(--muted-foreground)]">{label}</dt>
      <dd
        className={cn(
          "tabular-nums font-medium",
          tone === "ok" && "text-emerald-600 dark:text-emerald-300",
          tone === "warn" && "text-amber-600 dark:text-amber-300",
          tone === "bad" && "text-rose-600 dark:text-rose-300",
        )}
      >
        {value}
      </dd>
    </div>
  );
}

export function formatPercent(value: number | undefined): string {
  if (value === undefined || Number.isNaN(value)) return "—";
  return `${Math.round(value * 100)}%`;
}

export function confidenceTone(value: number | undefined): RowProps["tone"] {
  if (value === undefined) return undefined;
  if (value >= 0.85) return "ok";
  if (value >= 0.6) return "warn";
  return "bad";
}

export function policyLabel(policy: NodeAnnotations["policy"]): string {
  if (policy === "ask") return "Ask operator";
  if (policy === "auto") return "Autonomous";
  return "—";
}

export function formatRelative(iso: string | undefined): string {
  if (!iso) return "—";
  const t = Date.parse(iso);
  if (Number.isNaN(t)) return "—";
  const diffMs = Date.now() - t;
  const diffSec = Math.round(diffMs / 1000);
  if (diffSec < 60) return `${diffSec}s ago`;
  const diffMin = Math.round(diffSec / 60);
  if (diffMin < 60) return `${diffMin}m ago`;
  const diffHr = Math.round(diffMin / 60);
  if (diffHr < 48) return `${diffHr}h ago`;
  const diffDay = Math.round(diffHr / 24);
  return `${diffDay}d ago`;
}
