"use client";

import * as DialogPrimitive from "@radix-ui/react-dialog";
import { useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/Button";
import { cn } from "@/lib/utils";
import {
  RETRO_SECTION_KEYS,
  RETRO_SECTION_LABELS,
  summarizeMetricsLine,
  type Retro,
  type RetroSectionKey,
  type RetroSections,
} from "@/lib/api/retros";

/**
 * Worker-retrospective multi-step dialog. Five steps walk the
 * operator through the retro template:
 *
 *   1. Surface metrics — read-only counts aggregated server-side.
 *   2. What went well   — free-text.
 *   3. Surprises        — free-text.
 *   4. What to change   — free-text.
 *   5. Action items     — free-text.
 *
 * Each text step persists the operator's draft via `onSaveSections`
 * before advancing; the final step's "Finalize" button calls
 * `onFinalize` and locks the row. Once finalised the dialog rerenders
 * as a read-only summary.
 *
 * The component is intentionally container-free — the parent owns
 * the mutation cycle (TanStack Query handles invalidation in
 * `use-retros.ts`).
 */

export interface WorkerRetroDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** The retro under review. Pass `null` while the parent is creating
   *  one; the dialog renders a loading placeholder until the row
   *  resolves. */
  retro: Retro | null;
  workerName: string;
  /** Persist a partial sections update. Returns the promise so the
   *  dialog can hold the Next button in a pending state. */
  onSaveSections: (sections: Partial<RetroSections>) => void | Promise<unknown>;
  /** Finalise the retro. */
  onFinalize: () => void | Promise<unknown>;
}

const TOTAL_STEPS = RETRO_SECTION_KEYS.length;

export function WorkerRetroDialog({
  open,
  onOpenChange,
  retro,
  workerName,
  onSaveSections,
  onFinalize,
}: WorkerRetroDialogProps) {
  const [step, setStep] = useState(0);
  const [draft, setDraft] = useState<RetroSections>(() => emptyDraft());
  const [busy, setBusy] = useState(false);

  // Hydrate the draft when the retro arrives or the dialog opens
  // afresh. Closing the dialog resets the step pointer.
  useEffect(() => {
    if (open && retro) {
      setDraft({ ...emptyDraft(), ...retro.sections });
      setStep(0);
    }
    if (!open) {
      setStep(0);
      setBusy(false);
    }
  }, [open, retro]);

  const sectionKey = RETRO_SECTION_KEYS[step];
  const isMetricsStep = sectionKey === "surface_metrics";
  const isLastStep = step === TOTAL_STEPS - 1;
  const isFinalized = Boolean(retro?.finalized);

  const metricsLine = useMemo(() => {
    return retro ? summarizeMetricsLine(retro.metrics) : "";
  }, [retro]);

  const handleNext = async () => {
    if (busy || !retro) return;
    try {
      setBusy(true);
      if (!isMetricsStep) {
        // Persist the current section before advancing so a refresh
        // mid-retro doesn't lose the draft.
        await onSaveSections({ [sectionKey]: draft[sectionKey] } as Partial<RetroSections>);
      }
      setStep((s) => Math.min(s + 1, TOTAL_STEPS - 1));
    } finally {
      setBusy(false);
    }
  };

  const handleBack = () => {
    if (busy) return;
    setStep((s) => Math.max(s - 1, 0));
  };

  const handleFinalize = async () => {
    if (busy || !retro || isFinalized) return;
    try {
      setBusy(true);
      // Final save of the operator-fillable bucket; the gateway lock
      // happens via `onFinalize`.
      const payload: Partial<RetroSections> = {};
      for (const key of RETRO_SECTION_KEYS) {
        if (key !== "surface_metrics") payload[key] = draft[key];
      }
      await onSaveSections(payload);
      await onFinalize();
      onOpenChange(false);
    } finally {
      setBusy(false);
    }
  };

  return (
    <DialogPrimitive.Root open={open} onOpenChange={onOpenChange}>
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay
          className={cn(
            "fixed inset-0 z-40 bg-black/40 backdrop-blur-sm",
            "data-[state=open]:animate-in data-[state=closed]:animate-out",
          )}
        />
        <DialogPrimitive.Content
          aria-modal="true"
          data-testid="worker-retro-dialog"
          className={cn(
            "fixed left-1/2 top-1/2 z-50 w-[92vw] max-w-xl -translate-x-1/2 -translate-y-1/2",
            "rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--background)] p-6 shadow-lg",
            "focus:outline-none",
          )}
        >
          <DialogPrimitive.Title className="text-base font-semibold text-[var(--foreground)]">
            Retro with {workerName}
          </DialogPrimitive.Title>
          <DialogPrimitive.Description
            className="mt-1 text-sm text-[var(--muted-foreground)]"
            data-testid="retro-dialog-stepper"
          >
            Step {step + 1} of {TOTAL_STEPS} — {RETRO_SECTION_LABELS[sectionKey]}
          </DialogPrimitive.Description>

          {!retro ? (
            <p
              data-testid="retro-dialog-loading"
              className="mt-6 text-sm text-[var(--muted-foreground)]"
            >
              Preparing the retro…
            </p>
          ) : isFinalized ? (
            <FinalizedSummary retro={retro} />
          ) : isMetricsStep ? (
            <MetricsPanel retro={retro} metricsLine={metricsLine} />
          ) : (
            <SectionTextArea
              sectionKey={sectionKey}
              value={draft[sectionKey]}
              onChange={(value) =>
                setDraft((prev) => ({ ...prev, [sectionKey]: value }))
              }
              disabled={busy}
            />
          )}

          <div className="mt-6 flex items-center justify-between gap-2">
            <Button
              variant="ghost"
              data-testid="retro-dialog-back"
              onClick={handleBack}
              disabled={step === 0 || busy || isFinalized}
            >
              Back
            </Button>
            <div className="flex items-center gap-2">
              <Button
                variant="ghost"
                data-testid="retro-dialog-close"
                onClick={() => onOpenChange(false)}
                disabled={busy}
              >
                Close
              </Button>
              {isFinalized ? null : isLastStep ? (
                <Button
                  variant="primary"
                  data-testid="retro-dialog-finalize"
                  onClick={handleFinalize}
                  loading={busy}
                  disabled={busy}
                >
                  Finalize
                </Button>
              ) : (
                <Button
                  variant="primary"
                  data-testid="retro-dialog-next"
                  onClick={handleNext}
                  loading={busy}
                  disabled={busy}
                >
                  Next
                </Button>
              )}
            </div>
          </div>
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}

function emptyDraft(): RetroSections {
  return RETRO_SECTION_KEYS.reduce((acc, key) => {
    acc[key] = "";
    return acc;
  }, {} as RetroSections);
}

function MetricsPanel({
  retro,
  metricsLine,
}: {
  retro: Retro;
  metricsLine: string;
}) {
  const errorEntries = Object.entries(retro.metrics.error_count_by_category);
  return (
    <div className="mt-5 space-y-3" data-testid="retro-dialog-metrics">
      <p className="text-sm text-[var(--foreground)]" data-testid="retro-dialog-metrics-summary">
        {metricsLine}
      </p>
      <dl className="grid grid-cols-2 gap-x-6 gap-y-2 text-xs">
        <MetricRow label="Actions total" value={retro.metrics.actions_total} />
        <MetricRow label="Approved" value={retro.metrics.actions_approved} />
        <MetricRow label="Refused" value={retro.metrics.actions_refused} />
        <MetricRow label="Escalations" value={retro.metrics.escalations} />
        <MetricRow
          label="Skills learned"
          value={retro.metrics.learned_skills_added}
        />
      </dl>
      {errorEntries.length > 0 ? (
        <div
          className="rounded-[var(--radius-md)] border border-[var(--border)] p-3"
          data-testid="retro-dialog-error-summary"
        >
          <p className="text-xs font-medium text-[var(--foreground)]">
            Errors by category
          </p>
          <ul className="mt-2 space-y-1 text-xs text-[var(--muted-foreground)]">
            {errorEntries.map(([kind, count]) => (
              <li key={kind}>
                {kind}: {count}
              </li>
            ))}
          </ul>
        </div>
      ) : null}
    </div>
  );
}

function MetricRow({ label, value }: { label: string; value: number }) {
  return (
    <>
      <dt className="text-[var(--muted-foreground)]">{label}</dt>
      <dd className="text-right font-medium text-[var(--foreground)]">{value}</dd>
    </>
  );
}

function SectionTextArea({
  sectionKey,
  value,
  onChange,
  disabled,
}: {
  sectionKey: RetroSectionKey;
  value: string;
  onChange: (next: string) => void;
  disabled: boolean;
}) {
  return (
    <textarea
      data-testid={`retro-dialog-section-${sectionKey}`}
      aria-label={RETRO_SECTION_LABELS[sectionKey]}
      value={value}
      onChange={(e) => onChange(e.target.value)}
      rows={6}
      disabled={disabled}
      placeholder={`Capture ${RETRO_SECTION_LABELS[sectionKey].toLowerCase()}…`}
      className={cn(
        "mt-5 w-full resize-none rounded-[var(--radius-md)] border border-[var(--input)] bg-[var(--background)]",
        "px-3 py-2 text-sm leading-relaxed text-[var(--foreground)]",
        "focus:outline-none focus:ring-2 focus:ring-[var(--ring)]",
        "disabled:opacity-60",
      )}
    />
  );
}

function FinalizedSummary({ retro }: { retro: Retro }) {
  return (
    <div className="mt-5 space-y-3" data-testid="retro-dialog-finalized">
      <p className="text-sm text-[var(--muted-foreground)]">
        Retro finalised — sections are read-only.
      </p>
      <dl className="space-y-3 text-sm">
        {RETRO_SECTION_KEYS.filter((k) => k !== "surface_metrics").map((key) => (
          <div key={key}>
            <dt className="text-xs font-medium uppercase tracking-wide text-[var(--muted-foreground)]">
              {RETRO_SECTION_LABELS[key]}
            </dt>
            <dd
              className="mt-1 whitespace-pre-wrap text-[var(--foreground)]"
              data-testid={`retro-dialog-readonly-${key}`}
            >
              {retro.sections[key] || "—"}
            </dd>
          </div>
        ))}
      </dl>
    </div>
  );
}
