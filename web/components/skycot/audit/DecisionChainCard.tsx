"use client";

import { useState } from "react";
import { ChevronDown, ChevronRight } from "lucide-react";
import { RedactedText } from "@/components/skycot/RedactedText";
import { cn } from "@/lib/utils";
import type { RedactionPolicy } from "@/lib/redaction/redact";
import type { DecisionStep } from "@/lib/types/audit-detail";

/**
 * Decision-chain card — the 8-step trace per the wireframe (memory
 * loaded → skills considered → plan generated → plan selected →
 * policy check → approval → execution → outcome).
 *
 * Each step renders as a numbered heading + body. Long-form data
 * (alternative plans, raw memory hashes) lives in the optional
 * `details` payload on each step and renders inside an expandable
 * `<details>` block.
 */

export interface DecisionChainCardProps {
  steps: DecisionStep[];
  policy: RedactionPolicy;
}

export function DecisionChainCard({ steps, policy }: DecisionChainCardProps) {
  const ordered = [...steps].sort((a, b) => a.n - b.n);
  return (
    <section
      data-testid="audit-decision-chain"
      className="rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] p-5 space-y-3"
    >
      <header>
        <h2 className="text-sm font-semibold text-[var(--foreground)]">
          Decision chain
        </h2>
        <p className="text-xs text-[var(--muted-foreground)]">
          The 8-step trace this action followed.
        </p>
      </header>

      <ol className="space-y-2">
        {ordered.map((step) => (
          <DecisionRow key={step.n} step={step} policy={policy} />
        ))}
      </ol>
    </section>
  );
}

function DecisionRow({ step, policy }: { step: DecisionStep; policy: RedactionPolicy }) {
  const [open, setOpen] = useState(false);
  const hasDetails = step.details !== undefined && hasKeys(step.details);
  return (
    <li
      data-testid={`audit-decision-step-${step.n}`}
      data-step={step.n}
      className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)]/40 p-3"
    >
      <div className="flex items-start gap-3">
        <span
          aria-hidden
          className="inline-flex size-6 shrink-0 items-center justify-center rounded-full border border-[var(--border)] bg-[var(--muted)]/50 text-[11px] font-semibold tabular-nums text-[var(--muted-foreground)]"
        >
          {step.n}
        </span>
        <div className="min-w-0 flex-1 space-y-1">
          <p
            className="text-sm font-medium text-[var(--foreground)]"
            data-testid={`audit-decision-step-${step.n}-title`}
          >
            <RedactedText policy={policy}>{step.title}</RedactedText>
          </p>
          <p
            className="text-sm text-[var(--muted-foreground)] whitespace-pre-line"
            data-testid={`audit-decision-step-${step.n}-body`}
          >
            <RedactedText policy={policy}>{step.body}</RedactedText>
          </p>
          {hasDetails ? (
            <button
              type="button"
              data-testid={`audit-decision-step-${step.n}-toggle`}
              aria-expanded={open}
              onClick={() => setOpen((v) => !v)}
              className="mt-1 inline-flex items-center gap-1 text-[11px] font-medium text-[var(--primary)] hover:underline underline-offset-4"
            >
              {open ? <ChevronDown aria-hidden className="size-3" /> : <ChevronRight aria-hidden className="size-3" />}
              {open ? "Hide details" : "Show details"}
            </button>
          ) : null}
          {open && hasDetails ? (
            <pre
              data-testid={`audit-decision-step-${step.n}-details`}
              className={cn(
                "mt-2 max-h-64 overflow-auto rounded-[var(--radius-sm)] border border-[var(--border)] bg-[var(--muted)]/30 p-2",
                "font-mono text-[11px] text-[var(--foreground)] whitespace-pre",
              )}
            >
              {JSON.stringify(step.details, null, 2)}
            </pre>
          ) : null}
        </div>
      </div>
    </li>
  );
}

function hasKeys(obj: Record<string, unknown> | undefined): boolean {
  if (!obj) return false;
  return Object.keys(obj).length > 0;
}
