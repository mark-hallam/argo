"use client";

import { useMemo, useState } from "react";
import { ChevronDown, ChevronRight } from "lucide-react";
import { DECIDER_ROLE_LABELS, type DeciderRole } from "@/lib/types/audit";
import {
  computeRollbackLineDiff,
  summarizeStepChanges,
  type RollbackStepSummary,
  type WorkflowRollbackPayload,
} from "@/lib/diff/workflow-version-diff";

/**
 * Renders the rollback-specific block inside the audit-detail hero
 * for a `workflow_rolled_back` action:
 *
 *   1. Title line — "Rolled back v5 → v3 by Admin Mark"
 *   2. Summary line — "12 steps changed, 3 added, 5 modified, 4 removed"
 *   3. Expandable [Show diff] / [Hide diff] panel rendering the LCS
 *      line-diff from #1967's CLI logic, line-prefixed with `+`/`-`/` `.
 *
 * The component is read-only: no inline rollback button (the rollback
 * page from #1980 owns that surface). All copy uses the audit page's
 * restrained register — no gradient, no Audiowide, no badge chrome.
 */
export interface WorkflowRollbackDiffProps {
  payload: WorkflowRollbackPayload;
  /** Approver name from `AuditDetail.approver` — surfaced in the
   *  "by Admin Mark" tail when `decider_name` isn't on the payload. */
  approverName?: string;
}

export function WorkflowRollbackDiff({ payload, approverName }: WorkflowRollbackDiffProps) {
  const summary: RollbackStepSummary = useMemo(
    () => summarizeStepChanges(payload.before_toml, payload.after_toml),
    [payload.before_toml, payload.after_toml],
  );
  const diff = useMemo(() => computeRollbackLineDiff(payload), [payload]);
  const [expanded, setExpanded] = useState(false);

  const title = rollbackTitle(payload, approverName);
  const summaryLine = formatSummaryLine(summary);
  const hasDiffBodies = Boolean(payload.before_toml && payload.after_toml);

  return (
    <div
      data-testid="workflow-rollback-diff"
      data-from-version={payload.from_version}
      data-to-version={payload.to_version}
      className="space-y-2 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--muted)]/30 p-3"
    >
      <p
        data-testid="workflow-rollback-title"
        className="text-sm font-medium text-[var(--foreground)]"
      >
        {title}
      </p>
      <p
        data-testid="workflow-rollback-summary"
        className="text-xs text-[var(--muted-foreground)]"
      >
        {summaryLine}
      </p>

      {hasDiffBodies ? (
        <div>
          <button
            type="button"
            data-testid="workflow-rollback-toggle"
            aria-expanded={expanded}
            onClick={() => setExpanded((v) => !v)}
            className="inline-flex items-center gap-1 rounded-[var(--radius-sm)] border border-[var(--border)] bg-[var(--card)] px-2 py-1 text-xs font-medium text-[var(--foreground)] hover:bg-[var(--accent)]/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]"
          >
            {expanded ? (
              <ChevronDown aria-hidden className="size-3" />
            ) : (
              <ChevronRight aria-hidden className="size-3" />
            )}
            {expanded ? "Hide diff" : "Show diff"}
          </button>
          {expanded ? (
            <DiffPanel
              hunks={diff.hunks}
              unchanged={diff.unchanged}
              fromVersion={payload.from_version}
              toVersion={payload.to_version}
            />
          ) : null}
        </div>
      ) : null}
    </div>
  );
}

function DiffPanel({
  hunks,
  unchanged,
  fromVersion,
  toVersion,
}: {
  hunks: ReturnType<typeof computeRollbackLineDiff>["hunks"];
  unchanged: boolean;
  fromVersion?: number;
  toVersion?: number;
}) {
  if (unchanged) {
    return (
      <p
        data-testid="workflow-rollback-diff-empty"
        className="mt-2 text-xs text-[var(--muted-foreground)]"
      >
        No line-level differences between the two versions.
      </p>
    );
  }
  return (
    <pre
      data-testid="workflow-rollback-diff-panel"
      className="mt-2 max-h-80 overflow-auto rounded-[var(--radius-sm)] border border-[var(--border)] bg-[var(--card)] p-2 font-mono text-[12px] leading-5"
    >
      {hunks.map((hunk, hunkIdx) => (
        <div
          key={`${hunk.fromStart}-${hunk.toStart}-${hunkIdx}`}
          data-testid="workflow-rollback-diff-hunk"
        >
          <div className="text-[var(--muted-foreground)]">
            {`@@ -${hunk.fromStart},${hunk.fromLength} +${hunk.toStart},${hunk.toLength}` +
              hunkHeaderTail(fromVersion, toVersion)}
          </div>
          {hunk.lines.map((line, lineIdx) => (
            <div
              key={`${hunkIdx}-${lineIdx}`}
              data-kind={line.kind}
              className={lineClassName(line.kind)}
            >
              {linePrefix(line.kind)}
              {line.text}
            </div>
          ))}
        </div>
      ))}
    </pre>
  );
}

function hunkHeaderTail(fromVersion?: number, toVersion?: number): string {
  if (typeof fromVersion !== "number" || typeof toVersion !== "number") return " @@";
  return ` @@  v${fromVersion} → v${toVersion}`;
}

function linePrefix(kind: "context" | "added" | "removed"): string {
  if (kind === "added") return "+ ";
  if (kind === "removed") return "- ";
  return "  ";
}

function lineClassName(kind: "context" | "added" | "removed"): string {
  if (kind === "added") return "text-[var(--success)]";
  if (kind === "removed") return "text-[var(--destructive)]";
  return "text-[var(--muted-foreground)]";
}

function rollbackTitle(
  payload: WorkflowRollbackPayload,
  approverName: string | undefined,
): string {
  const versions = formatVersionArrow(payload.from_version, payload.to_version);
  const decider = formatDecider(payload.decider_role, payload.decider_name ?? approverName);
  if (versions && decider) return `Rolled back ${versions} by ${decider}`;
  if (versions) return `Rolled back ${versions}`;
  if (decider) return `Rolled back by ${decider}`;
  return "Workflow rolled back";
}

function formatVersionArrow(from: number | undefined, to: number | undefined): string {
  if (typeof from === "number" && typeof to === "number") return `v${from} → v${to}`;
  if (typeof from === "number") return `from v${from}`;
  if (typeof to === "number") return `to v${to}`;
  return "";
}

function formatDecider(role: DeciderRole | undefined, name: string | undefined): string {
  if (!name) return "";
  if (!role) return name;
  return `${DECIDER_ROLE_LABELS[role]} ${name}`;
}

function formatSummaryLine(summary: RollbackStepSummary): string {
  if (summary.total === 0) return "No steps changed.";
  const stepWord = summary.total === 1 ? "step" : "steps";
  return `${summary.total} ${stepWord} changed, ${summary.added} added, ${summary.modified} modified, ${summary.removed} removed`;
}
