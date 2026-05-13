"use client";

import { useId } from "react";
import type { ExportScope } from "@/lib/sample-export";

/**
 * Scope radio for the evidence-bundle exporter. Three primary options:
 *  - Current filtered view
 *  - Specific time range  → renders from/to date pickers
 *  - Sampled audit selection → renders sample-size number input
 *
 * Lock modes for E5-c (single-action) and E5-d (replay) hide the
 * radio entirely and surface a static read-only summary instead —
 * those entry points pre-narrow the scope server-side.
 */

export type ScopeKind = ExportScope["kind"];
export const PRIMARY_SCOPE_KINDS = ["filtered", "range", "sampled"] as const;
export type PrimaryScopeKind = (typeof PRIMARY_SCOPE_KINDS)[number];

const SCOPE_LABELS: Record<PrimaryScopeKind, string> = {
  filtered: "Current filtered view",
  range: "Specific time range",
  sampled: "Sampled audit selection",
};

export interface ExportScopeSelectorProps {
  scope: ExportScope;
  onChange: (next: ExportScope) => void;
  disabled?: boolean;
}

export function ExportScopeSelector({
  scope,
  onChange,
  disabled = false,
}: ExportScopeSelectorProps) {
  const groupId = useId();

  if (scope.kind === "single-action") {
    return (
      <LockedScope
        label="Single action"
        detail={`Action ${scope.actionId} ± 1 minute`}
      />
    );
  }
  if (scope.kind === "replay") {
    return (
      <LockedScope
        label="Replay artifact"
        detail={`Replay ${scope.replayId}`}
      />
    );
  }

  return (
    <fieldset
      className="space-y-2"
      data-testid="export-scope-selector"
      aria-describedby={`${groupId}-help`}
    >
      <legend className="text-xs font-medium uppercase tracking-wide text-[var(--muted-foreground)]">
        Scope
      </legend>
      <div className="space-y-1.5" role="radiogroup" aria-label="Export scope">
        {PRIMARY_SCOPE_KINDS.map((kind) => (
          <label
            key={kind}
            data-testid={`export-scope-option-${kind}`}
            className="flex items-center gap-2 rounded-[var(--radius-sm)] border border-[var(--border)] px-3 py-2 text-sm hover:bg-[var(--accent)]/30 has-[input:checked]:border-[var(--primary)] has-[input:checked]:bg-[var(--primary)]/10"
          >
            <input
              type="radio"
              name="export-scope"
              value={kind}
              checked={scope.kind === kind}
              disabled={disabled}
              onChange={() => onChange(defaultScopeFor(kind))}
              className="size-4 accent-[var(--primary)]"
            />
            <span className="text-[var(--foreground)]">{SCOPE_LABELS[kind]}</span>
          </label>
        ))}
      </div>

      {scope.kind === "range" ? (
        <RangeInputs scope={scope} onChange={onChange} disabled={disabled} />
      ) : null}

      {scope.kind === "sampled" ? (
        <SampledInput scope={scope} onChange={onChange} disabled={disabled} />
      ) : null}
    </fieldset>
  );
}

function defaultScopeFor(kind: PrimaryScopeKind): ExportScope {
  if (kind === "filtered") return { kind: "filtered" };
  if (kind === "sampled") return { kind: "sampled", size: 10 };
  const now = new Date();
  const weekAgo = new Date(now.getTime() - 7 * 86_400_000);
  return {
    kind: "range",
    from: weekAgo.toISOString().slice(0, 10),
    to: now.toISOString().slice(0, 10),
  };
}

function RangeInputs({
  scope,
  onChange,
  disabled,
}: {
  scope: Extract<ExportScope, { kind: "range" }>;
  onChange: (next: ExportScope) => void;
  disabled: boolean;
}) {
  return (
    <div className="grid grid-cols-2 gap-2 pt-1" data-testid="export-scope-range-inputs">
      <label className="space-y-1 text-xs text-[var(--muted-foreground)]">
        <span>From</span>
        <input
          type="date"
          value={scope.from}
          disabled={disabled}
          data-testid="export-scope-range-from"
          onChange={(e) =>
            onChange({ kind: "range", from: e.target.value, to: scope.to })
          }
          className="w-full h-9 px-2 rounded-[var(--radius-sm)] border border-[var(--input)] bg-[var(--card)]/60 text-sm text-[var(--foreground)] focus:outline-none focus:ring-2 focus:ring-[var(--ring)]"
        />
      </label>
      <label className="space-y-1 text-xs text-[var(--muted-foreground)]">
        <span>To</span>
        <input
          type="date"
          value={scope.to}
          disabled={disabled}
          data-testid="export-scope-range-to"
          onChange={(e) =>
            onChange({ kind: "range", from: scope.from, to: e.target.value })
          }
          className="w-full h-9 px-2 rounded-[var(--radius-sm)] border border-[var(--input)] bg-[var(--card)]/60 text-sm text-[var(--foreground)] focus:outline-none focus:ring-2 focus:ring-[var(--ring)]"
        />
      </label>
    </div>
  );
}

function SampledInput({
  scope,
  onChange,
  disabled,
}: {
  scope: Extract<ExportScope, { kind: "sampled" }>;
  onChange: (next: ExportScope) => void;
  disabled: boolean;
}) {
  return (
    <label
      className="block space-y-1 pt-1 text-xs text-[var(--muted-foreground)]"
      data-testid="export-scope-sampled-input"
    >
      <span>Sample size</span>
      <input
        type="number"
        min={1}
        max={1_000}
        value={scope.size}
        disabled={disabled}
        onChange={(e) =>
          onChange({
            kind: "sampled",
            size: Math.max(1, Number.parseInt(e.target.value || "1", 10)),
          })
        }
        className="w-32 h-9 px-2 rounded-[var(--radius-sm)] border border-[var(--input)] bg-[var(--card)]/60 text-sm text-[var(--foreground)] focus:outline-none focus:ring-2 focus:ring-[var(--ring)]"
      />
    </label>
  );
}

function LockedScope({ label, detail }: { label: string; detail: string }) {
  return (
    <div
      data-testid="export-scope-locked"
      className="space-y-1 rounded-[var(--radius-sm)] border border-[var(--border)] bg-[var(--card)]/40 px-3 py-2 text-sm"
    >
      <p className="text-xs font-medium uppercase tracking-wide text-[var(--muted-foreground)]">
        Scope
      </p>
      <p className="font-medium text-[var(--foreground)]">{label}</p>
      <p className="text-xs text-[var(--muted-foreground)]">{detail}</p>
    </div>
  );
}
