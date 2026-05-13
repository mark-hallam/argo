"use client";

import Link from "next/link";
import { ArrowLeftCircle, Clock3, GitMerge, Mail } from "lucide-react";
import { RedactedText } from "@/components/skycot/RedactedText";
import type { RedactionPolicy } from "@/lib/redaction/redact";
import type { AuditDetailInputs, AuditTriggerKind } from "@/lib/types/audit-detail";

/**
 * Inputs card — what triggered this action and any predecessor link.
 * The predecessor link routes to its own drill-in surface preserving
 * the audit-feed back-stack via the browser.
 */

export interface ActionInputsCardProps {
  inputs: AuditDetailInputs;
  policy: RedactionPolicy;
}

export function ActionInputsCard({ inputs, policy }: ActionInputsCardProps) {
  const Icon = ICONS[inputs.triggerKind];
  return (
    <section
      data-testid="audit-inputs-card"
      data-trigger-kind={inputs.triggerKind}
      className="rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] p-5 space-y-3"
    >
      <header className="flex items-center justify-between">
        <h2 className="text-sm font-semibold text-[var(--foreground)]">Inputs</h2>
        <span className="text-[11px] uppercase tracking-wider text-[var(--muted-foreground)]">
          {KIND_LABEL[inputs.triggerKind]}
        </span>
      </header>

      <div className="flex items-start gap-3 text-sm">
        <Icon
          aria-hidden
          className="size-4 mt-0.5 shrink-0 text-[var(--muted-foreground)]"
        />
        <p
          data-testid="audit-inputs-summary"
          className="text-[var(--foreground)]"
        >
          <RedactedText policy={policy}>{inputs.triggerSummary}</RedactedText>
        </p>
      </div>

      {inputs.predecessor ? (
        <div className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--muted)]/30 p-3">
          <p className="mb-1 text-[11px] uppercase tracking-wider text-[var(--muted-foreground)]">
            Predecessor action
          </p>
          <Link
            href={`/audit/action/${encodeURIComponent(inputs.predecessor.id)}`}
            data-testid="audit-inputs-predecessor"
            data-predecessor-id={inputs.predecessor.id}
            className="inline-flex items-center gap-2 text-sm text-[var(--primary)] underline-offset-4 hover:underline"
          >
            <ArrowLeftCircle aria-hidden className="size-4" />
            <span>
              <RedactedText policy={policy}>{inputs.predecessor.verb}</RedactedText>
            </span>
          </Link>
          <p className="mt-1 font-mono text-[11px] text-[var(--muted-foreground)]">
            {inputs.predecessor.id}
          </p>
        </div>
      ) : null}
    </section>
  );
}

const KIND_LABEL: Record<AuditTriggerKind, string> = {
  message: "Message",
  cron: "Cron",
  dependent_action: "Dependent action",
};

const ICONS: Record<AuditTriggerKind, typeof Mail> = {
  message: Mail,
  cron: Clock3,
  dependent_action: GitMerge,
};
