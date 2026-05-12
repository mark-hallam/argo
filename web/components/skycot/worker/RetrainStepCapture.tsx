"use client";

import { useEffect, useRef } from "react";
import { Button } from "@/components/ui/Button";
import { cn } from "@/lib/utils";

/**
 * Retrain Step 1 — Capture.
 *
 * Free-text textarea (≥ 4 rows) for the delegator's plain-English
 * correction. Continue is disabled until ≥ 10 non-whitespace
 * characters land in the field (matches the journey-retrain spec).
 */

export interface RetrainStepCaptureProps {
  workerName: string;
  triggerContext: "charter" | "voice" | "audit";
  value: string;
  onChange: (next: string) => void;
  onContinue: () => void;
  onCancel: () => void;
}

const MIN_CHARS = 10;

function headingFor(ctx: RetrainStepCaptureProps["triggerContext"], name: string): string {
  switch (ctx) {
    case "voice":
      return `Adjust how ${name} sounds`;
    case "audit":
      return `Make sure this doesn't happen again`;
    case "charter":
    default:
      return `Adjust how ${name} behaves`;
  }
}

function placeholderFor(ctx: RetrainStepCaptureProps["triggerContext"]): string {
  switch (ctx) {
    case "voice":
      return "e.g. Sound a little warmer in first replies. Use 'guests' not 'customers'.";
    case "audit":
      return "e.g. Never ship to PO boxes. If asked, escalate to Marta.";
    case "charter":
    default:
      return "e.g. Refunds under $50 auto-approve. Above $50 ask Marta first.";
  }
}

export function RetrainStepCapture({
  workerName,
  triggerContext,
  value,
  onChange,
  onContinue,
  onCancel,
}: RetrainStepCaptureProps) {
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);

  useEffect(() => {
    if (textareaRef.current) textareaRef.current.focus();
  }, []);

  const trimmed = value.trim();
  const disabled = trimmed.length < MIN_CHARS;

  return (
    <div data-testid="retrain-step-capture" className="space-y-4">
      <header className="space-y-1">
        <h2 className="text-base font-semibold text-[var(--foreground)]">
          {headingFor(triggerContext, workerName)}
        </h2>
        <p className="text-sm text-[var(--muted-foreground)]">
          What do you want {workerName} to do differently? Plain English
          works best — she&apos;ll figure out where the change belongs.
        </p>
      </header>

      <textarea
        ref={textareaRef}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        rows={5}
        data-testid="retrain-input"
        aria-label="Plain English correction"
        placeholder={placeholderFor(triggerContext)}
        className={cn(
          "w-full resize-none rounded-[var(--radius-md)] border border-[var(--input)] bg-[var(--background)]",
          "px-3 py-2 text-sm leading-relaxed text-[var(--foreground)]",
          "focus:outline-none focus:ring-2 focus:ring-[var(--ring)]",
        )}
      />

      <div className="flex items-center justify-between text-xs text-[var(--muted-foreground)]">
        <span data-testid="retrain-input-counter">
          {trimmed.length} / {MIN_CHARS} characters minimum
        </span>
      </div>

      <div className="flex justify-end gap-2">
        <Button variant="ghost" data-testid="retrain-cancel" onClick={onCancel}>
          Cancel
        </Button>
        <Button
          variant="primary"
          data-testid="retrain-continue"
          onClick={onContinue}
          disabled={disabled}
        >
          Continue
        </Button>
      </div>
    </div>
  );
}
