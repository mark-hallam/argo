"use client";

import { useId } from "react";
import {
  EXPORT_FORMAT_DESCRIPTIONS,
  EXPORT_FORMAT_LABELS,
  EXPORT_FORMATS,
  type ExportFormat,
} from "@/lib/sample-export";

/**
 * Format radio for the evidence-bundle exporter. Renders 4 options
 * (SOC2 Type II default, GDPR Article 15, HIPAA disclosure log,
 * Generic JSON + PDF) plus the per-format description copy required
 * by the brief acceptance criteria.
 */
export interface ExportFormatSelectorProps {
  value: ExportFormat;
  onChange: (next: ExportFormat) => void;
  disabled?: boolean;
}

export function ExportFormatSelector({
  value,
  onChange,
  disabled = false,
}: ExportFormatSelectorProps) {
  const groupId = useId();
  return (
    <fieldset
      className="space-y-2"
      data-testid="export-format-selector"
      aria-describedby={`${groupId}-help`}
    >
      <legend className="text-xs font-medium uppercase tracking-wide text-[var(--muted-foreground)]">
        Format
      </legend>
      <div className="space-y-1.5" role="radiogroup" aria-label="Export format">
        {EXPORT_FORMATS.map((format) => (
          <FormatOption
            key={format}
            format={format}
            selected={value === format}
            disabled={disabled}
            onSelect={() => onChange(format)}
          />
        ))}
      </div>
      <p
        id={`${groupId}-help`}
        data-testid="export-format-help"
        className="rounded-[var(--radius-sm)] bg-[var(--card)]/60 px-3 py-2 text-xs text-[var(--muted-foreground)]"
      >
        {EXPORT_FORMAT_DESCRIPTIONS[value]}
      </p>
    </fieldset>
  );
}

interface FormatOptionProps {
  format: ExportFormat;
  selected: boolean;
  disabled: boolean;
  onSelect: () => void;
}

function FormatOption({ format, selected, disabled, onSelect }: FormatOptionProps) {
  return (
    <label
      data-testid={`export-format-option-${format}`}
      data-selected={selected || undefined}
      className="flex items-start gap-2 rounded-[var(--radius-sm)] border border-[var(--border)] px-3 py-2 text-sm hover:bg-[var(--accent)]/30 has-[input:checked]:border-[var(--primary)] has-[input:checked]:bg-[var(--primary)]/10"
    >
      <input
        type="radio"
        name="export-format"
        value={format}
        checked={selected}
        disabled={disabled}
        onChange={onSelect}
        className="mt-0.5 size-4 accent-[var(--primary)]"
      />
      <span className="font-medium text-[var(--foreground)]">
        {EXPORT_FORMAT_LABELS[format]}
      </span>
    </label>
  );
}
