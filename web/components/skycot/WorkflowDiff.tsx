/**
 * Pure-render workflow-version diff. Renders side-by-side on ≥ md
 * viewports and stacks to a single column on narrow screens.
 *
 * Inputs are pre-fetched TOML strings + version metadata; the
 * component performs the diff computation in-place but never reaches
 * for network, storage, or auth. Safe to render from RSC or client.
 */

import { GitCompareArrows } from "lucide-react";
import { compute_unified, type DiffLine, type DiffHunk } from "@/lib/diff/unified";
import { cn } from "@/lib/utils";

export interface VersionMeta {
  id: string;
  label: string;
  createdAt?: string;
}

export interface WorkflowDiffProps {
  fromToml: string;
  toToml: string;
  fromMeta: VersionMeta;
  toMeta: VersionMeta;
}

export function WorkflowDiff({ fromToml, toToml, fromMeta, toMeta }: WorkflowDiffProps) {
  const diff = compute_unified(fromToml, toToml);

  return (
    <section data-testid="workflow-diff" className="space-y-4">
      <header className="flex flex-col gap-2 rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] p-4 sm:flex-row sm:items-center sm:justify-between">
        <div className="min-w-0">
          <div className="flex items-center gap-2 text-sm font-medium text-[var(--muted-foreground)]">
            <GitCompareArrows className="size-4" aria-hidden="true" />
            <span>Workflow diff</span>
          </div>
          <h1 className="mt-1 text-xl font-semibold text-[var(--foreground)]">
            {fromMeta.label} <span className="text-[var(--muted-foreground)]">→</span>{" "}
            {toMeta.label}
          </h1>
        </div>
        <DiffStats hunks={diff.hunks} unchanged={diff.unchanged} />
      </header>

      {diff.unchanged ? (
        <EmptyState fromLabel={fromMeta.label} toLabel={toMeta.label} />
      ) : (
        <div className="space-y-3">
          {diff.hunks.map((hunk, idx) => (
            <HunkBlock
              key={`${hunk.fromStart}-${hunk.toStart}-${idx}`}
              hunk={hunk}
              fromLabel={fromMeta.label}
              toLabel={toMeta.label}
            />
          ))}
        </div>
      )}
    </section>
  );
}

function DiffStats({ hunks, unchanged }: { hunks: DiffHunk[]; unchanged: boolean }) {
  if (unchanged) {
    return (
      <p data-testid="workflow-diff-stats" className="text-sm text-[var(--muted-foreground)]">
        No changes
      </p>
    );
  }
  let added = 0;
  let removed = 0;
  for (const hunk of hunks) {
    for (const line of hunk.lines) {
      if (line.kind === "added") added++;
      else if (line.kind === "removed") removed++;
    }
  }
  return (
    <p data-testid="workflow-diff-stats" className="text-sm text-[var(--muted-foreground)]">
      <span className="text-emerald-700 dark:text-emerald-400">+{added}</span>{" "}
      <span className="text-rose-700 dark:text-rose-400">−{removed}</span>{" "}
      <span>across {hunks.length} {hunks.length === 1 ? "hunk" : "hunks"}</span>
    </p>
  );
}

function EmptyState({ fromLabel, toLabel }: { fromLabel: string; toLabel: string }) {
  return (
    <div
      data-testid="workflow-diff-empty"
      role="status"
      className="rounded-[var(--radius-lg)] border border-dashed border-[var(--border)] bg-[var(--card)] py-12 px-6 text-center"
    >
      <p className="text-sm text-[var(--muted-foreground)]">
        {fromLabel} and {toLabel} are identical (no changes).
      </p>
    </div>
  );
}

function HunkBlock({
  hunk,
  fromLabel,
  toLabel,
}: {
  hunk: DiffHunk;
  fromLabel: string;
  toLabel: string;
}) {
  const header = `@@ -${hunk.fromStart},${hunk.fromLength} +${hunk.toStart},${hunk.toLength} @@`;
  const ariaLabel = `Diff hunk from ${fromLabel} line ${hunk.fromStart} to ${toLabel} line ${hunk.toStart}`;
  return (
    <div
      role="region"
      aria-label={ariaLabel}
      data-testid="workflow-diff-hunk"
      className="overflow-hidden rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)]"
    >
      <div className="border-b border-[var(--border)] bg-[var(--muted)]/40 px-3 py-1 font-mono text-[11px] text-[var(--muted-foreground)]">
        {header}
      </div>
      {/* Side-by-side on md+, stacked on narrow screens. */}
      <div className="hidden md:grid md:grid-cols-2 md:divide-x md:divide-[var(--border)]">
        <SideColumn lines={hunk.lines} side="from" label={fromLabel} />
        <SideColumn lines={hunk.lines} side="to" label={toLabel} />
      </div>
      <div className="md:hidden">
        <UnifiedColumn lines={hunk.lines} />
      </div>
    </div>
  );
}

function SideColumn({
  lines,
  side,
  label,
}: {
  lines: DiffLine[];
  side: "from" | "to";
  label: string;
}) {
  return (
    <div data-testid={`workflow-diff-side-${side}`} className="font-mono text-xs">
      <div className="sr-only">{label}</div>
      <ol className="m-0 list-none p-0">
        {lines.map((line, idx) => (
          <SideRow key={`${side}-${idx}`} line={line} side={side} />
        ))}
      </ol>
    </div>
  );
}

function SideRow({ line, side }: { line: DiffLine; side: "from" | "to" }) {
  const lineNumber = side === "from" ? line.fromLine : line.toLine;
  const visible =
    side === "from" ? line.kind !== "added" : line.kind !== "removed";
  const tone = toneFor(line.kind);
  if (!visible) {
    return (
      <li
        aria-hidden="true"
        className="flex border-b border-[var(--border)]/40 bg-[var(--muted)]/20"
      >
        <span className="w-10 shrink-0 select-none border-r border-[var(--border)]/40 py-0.5 pr-2 text-right text-[var(--muted-foreground)]" />
        <span className="block min-h-[1.25rem] flex-1 px-3 py-0.5">&nbsp;</span>
      </li>
    );
  }
  return (
    <li className={cn("flex border-b border-[var(--border)]/40", tone.row)}>
      <span
        aria-hidden="true"
        className="w-10 shrink-0 select-none border-r border-[var(--border)]/40 py-0.5 pr-2 text-right text-[var(--muted-foreground)]"
      >
        {lineNumber ?? ""}
      </span>
      <span className={cn("flex-1 whitespace-pre-wrap break-all px-3 py-0.5", tone.text)}>
        {line.text || " "}
      </span>
    </li>
  );
}

function UnifiedColumn({ lines }: { lines: DiffLine[] }) {
  return (
    <ol data-testid="workflow-diff-unified" className="m-0 list-none p-0 font-mono text-xs">
      {lines.map((line, idx) => {
        const tone = toneFor(line.kind);
        const marker = line.kind === "added" ? "+" : line.kind === "removed" ? "-" : " ";
        return (
          <li key={`u-${idx}`} className={cn("flex border-b border-[var(--border)]/40", tone.row)}>
            <span
              aria-hidden="true"
              className="w-10 shrink-0 select-none border-r border-[var(--border)]/40 py-0.5 pr-2 text-right text-[var(--muted-foreground)]"
            >
              {line.fromLine ?? line.toLine ?? ""}
            </span>
            <span className={cn("flex-1 whitespace-pre-wrap break-all px-3 py-0.5", tone.text)}>
              <span aria-hidden="true" className="select-none pr-1">
                {marker}
              </span>
              {line.text || " "}
            </span>
          </li>
        );
      })}
    </ol>
  );
}

function toneFor(kind: DiffLine["kind"]): { row: string; text: string } {
  switch (kind) {
    case "added":
      return {
        row: "bg-emerald-50 dark:bg-emerald-950/30",
        text: "text-emerald-900 dark:text-emerald-100",
      };
    case "removed":
      return {
        row: "bg-rose-50 dark:bg-rose-950/30",
        text: "text-rose-900 dark:text-rose-100",
      };
    default:
      return { row: "", text: "text-[var(--foreground)]" };
  }
}
