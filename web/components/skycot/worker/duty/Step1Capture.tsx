"use client";

import { useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/Button";
import { cn } from "@/lib/utils";

/**
 * AddResponsibility Step 1 — textarea + 5 cron-preset chips.
 * Continue enables once ≥ 10 chars + (preset selected OR custom
 * cron parses). Single-select chips; "Custom…" opens an inline
 * cron input. Container holds canonical input + cronSpec state.
 */

export interface CronPreset {
  id: string;
  label: string;
  spec: string;
}

export const CRON_PRESETS: CronPreset[] = [
  { id: "morning", label: "Every morning", spec: "0 6 * * *" },
  { id: "hourly", label: "Every hour", spec: "0 * * * *" },
  { id: "weekdays-9", label: "Weekdays 9am", spec: "0 9 * * 1-5" },
  { id: "weekly-mon", label: "Once a week (Mon)", spec: "0 9 * * 1" },
];

const MIN_CHARS = 10;
const CRON_FIELD = /^(\*|\d+|\d+-\d+|\*\/\d+|(\d+(,\d+)+)|\d+-\d+\/\d+)$/;

/** Tiny cron syntax check — 5 space-separated fields, each well-formed. */
export function isValidCron(spec: string): boolean {
  const parts = spec.trim().split(/\s+/);
  if (parts.length !== 5) return false;
  return parts.every((p) => CRON_FIELD.test(p));
}

export interface Step1CaptureProps {
  workerName: string;
  value: string;
  cronSpec: string;
  presetId: string | null;
  onChangeValue: (next: string) => void;
  onSelectPreset: (preset: CronPreset | null, customSpec: string) => void;
  onContinue: () => void;
  onCancel: () => void;
}

export function Step1Capture({
  workerName,
  value,
  cronSpec,
  presetId,
  onChangeValue,
  onSelectPreset,
  onContinue,
  onCancel,
}: Step1CaptureProps) {
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const [customMode, setCustomMode] = useState(
    presetId === null && cronSpec.length > 0,
  );

  useEffect(() => {
    if (textareaRef.current) textareaRef.current.focus();
  }, []);

  const trimmed = value.trim();
  const cronValid = isValidCron(cronSpec);
  const continueDisabled = trimmed.length < MIN_CHARS || !cronValid;

  const handleSelect = (preset: CronPreset) => {
    setCustomMode(false);
    onSelectPreset(preset, preset.spec);
  };

  const handleCustom = () => {
    setCustomMode(true);
    onSelectPreset(null, presetId ? "" : cronSpec);
  };

  return (
    <div data-testid="duty-step-capture" className="space-y-4">
      <header className="space-y-1">
        <h2 className="text-base font-semibold text-[var(--foreground)]">
          Give {workerName} a new responsibility
        </h2>
        <p className="text-sm text-[var(--muted-foreground)]">
          Plain English. Pick how often it runs below.
        </p>
      </header>

      <textarea
        ref={textareaRef}
        value={value}
        onChange={(e) => onChangeValue(e.target.value)}
        rows={4}
        data-testid="duty-input"
        aria-label="New responsibility"
        placeholder={`What else do you want ${workerName} to handle?`}
        className={cn(
          "w-full resize-none rounded-[var(--radius-md)] border border-[var(--input)] bg-[var(--background)]",
          "px-3 py-2 text-sm leading-relaxed text-[var(--foreground)]",
          "focus:outline-none focus:ring-2 focus:ring-[var(--ring)]",
        )}
      />

      <div className="space-y-2">
        <p id="duty-cron-presets-label" className="text-xs font-medium uppercase tracking-wide text-[var(--muted-foreground)]">
          When should it run?
        </p>
        <div role="radiogroup" aria-labelledby="duty-cron-presets-label" data-testid="duty-cron-presets" className="flex flex-wrap gap-2">
          {CRON_PRESETS.map((p) => (
            <ChipButton key={p.id} testId={`duty-cron-preset-${p.id}`} active={!customMode && presetId === p.id} onClick={() => handleSelect(p)}>
              {p.label}
            </ChipButton>
          ))}
          <ChipButton testId="duty-cron-preset-custom" active={customMode} onClick={handleCustom}>Custom…</ChipButton>
        </div>

        {customMode ? (
          <div className="space-y-1">
            <label htmlFor="duty-cron-custom" className="text-xs text-[var(--muted-foreground)]">
              Cron expression (5 fields)
            </label>
            <input
              id="duty-cron-custom"
              data-testid="duty-cron-custom-input"
              value={cronSpec}
              onChange={(e) => onSelectPreset(null, e.target.value)}
              placeholder="e.g. 30 8 * * 1-5"
              aria-invalid={cronSpec.length > 0 && !cronValid}
              className={cn(
                "w-full rounded-[var(--radius-md)] border bg-[var(--background)] px-3 py-1.5 text-sm",
                "focus:outline-none focus:ring-2 focus:ring-[var(--ring)]",
                cronSpec.length > 0 && !cronValid ? "border-[var(--destructive)]" : "border-[var(--input)]",
              )}
            />
            {cronSpec.length > 0 && !cronValid ? (
              <p data-testid="duty-cron-custom-error" className="text-xs text-[var(--destructive-foreground)]">
                Doesn&apos;t look like a 5-field cron expression.
              </p>
            ) : null}
          </div>
        ) : null}
      </div>

      <div className="flex justify-end gap-2 pt-2">
        <Button variant="ghost" data-testid="duty-cancel" onClick={onCancel}>
          Cancel
        </Button>
        <Button
          variant="primary"
          data-testid="duty-continue"
          onClick={onContinue}
          disabled={continueDisabled}
        >
          Continue
        </Button>
      </div>
    </div>
  );
}

function ChipButton({
  testId,
  active,
  onClick,
  children,
}: {
  testId: string;
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      role="radio"
      aria-checked={active}
      data-testid={testId}
      data-active={active || undefined}
      onClick={onClick}
      className={cn(
        "rounded-full border px-3 py-1 text-xs transition-colors",
        "focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]",
        active
          ? "border-[var(--primary)] bg-[var(--primary)]/10 text-[var(--foreground)]"
          : "border-[var(--border)] bg-[var(--background)] text-[var(--muted-foreground)] hover:bg-[var(--accent)]/40",
      )}
    >
      {children}
    </button>
  );
}
