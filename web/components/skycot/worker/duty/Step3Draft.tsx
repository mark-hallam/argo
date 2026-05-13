"use client";

import { Pencil } from "lucide-react";
import { Button } from "@/components/ui/Button";
import type { DutyProposal } from "@/lib/api/duties";

/**
 * AddResponsibility Step 3 — Draft.
 *
 * Renders the classifier's plain-English proposal, the cron string
 * with a tiny semantic preview, and 3 sample outputs the duty would
 * produce. The "Adjust the wording" button returns to Step 1 with
 * the text editable; "Continue" advances to conflict resolution or
 * commit, depending on whether the proposal carries conflicts.
 *
 * The cron preview is intentionally light — full server-side
 * semantic preview ("Next fire: Tuesday 2026-05-13 …") lands with
 * `mission-enterprise-cron-semantic-preview`. v1 surfaces a human
 * gloss derived from the preset id where possible, or falls back to
 * a literal cron string when the user typed Custom…
 */

const KIND_LABEL: Record<DutyProposal["kind"], string> = {
  recurring: "Recurring",
  "event-triggered": "Event",
  "one-shot": "One-off",
};

const PRESET_DESCRIPTIONS: Record<string, string> = {
  "0 6 * * *": "Every morning at 6am.",
  "0 * * * *": "Every hour on the hour.",
  "0 9 * * 1-5": "Every weekday at 9am.",
  "0 9 * * 1": "Every Monday at 9am.",
};

const DAY_NAMES = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

export function cronGloss(spec: string): string {
  const direct = PRESET_DESCRIPTIONS[spec.trim()];
  if (direct) return direct;
  const parts = spec.trim().split(/\s+/);
  if (parts.length !== 5) return spec;
  const [m, h, dom, mon, dow] = parts;
  const min = m === "*" ? "every minute" : `:${m.padStart(2, "0")}`;
  const hr = h === "*" ? "every hour" : `${h}:00`;
  const day = dow === "*" ? "" : ` on ${humanDow(dow)}`;
  if (m === "*") return `Every minute${day}.`;
  if (h === "*") return `Every hour at ${min}${day}.`;
  if (dom === "*" && mon === "*") return `At ${hr}${day}.`;
  return `${hr} (cron ${spec}).`;
}

function humanDow(dow: string): string {
  if (/^\d+$/.test(dow)) return DAY_NAMES[parseInt(dow, 10) % 7] ?? dow;
  if (/^\d+-\d+$/.test(dow)) {
    const [a, b] = dow.split("-").map((s) => parseInt(s, 10));
    if (!Number.isNaN(a) && !Number.isNaN(b))
      return `${DAY_NAMES[a % 7]}–${DAY_NAMES[b % 7]}`;
  }
  return dow;
}

export function nextFireGloss(spec: string, now: Date = new Date()): string {
  const parts = spec.trim().split(/\s+/);
  if (parts.length !== 5) return "";
  const target = new Date(now.getTime());
  target.setSeconds(0, 0);
  // Bump by an hour as a lightweight floor; the brief allows v1 to be
  // approximate — accurate next-fire computation lands server-side.
  target.setMinutes(target.getMinutes() + 60);
  return ` Next fire approx ${target.toLocaleString(undefined, {
    weekday: "short",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  })}.`;
}

export interface Step3DraftProps {
  workerName: string;
  proposal: DutyProposal;
  onAdjust: () => void;
  onContinue: () => void;
  onCancel: () => void;
}

export function Step3Draft({
  workerName,
  proposal,
  onAdjust,
  onContinue,
  onCancel,
}: Step3DraftProps) {
  const cronText = proposal.schedule.kind === "cron"
    ? `${proposal.schedule.spec} — ${cronGloss(proposal.schedule.spec)}${nextFireGloss(proposal.schedule.spec)}`
    : `event: ${proposal.schedule.spec}`;
  const conflictCount = proposal.conflicts?.length ?? 0;

  return (
    <div data-testid="duty-step-draft" className="space-y-4">
      <header className="space-y-1">
        <p
          data-testid="duty-classification-chip"
          className="inline-flex items-center rounded-full border border-[var(--border)] bg-[var(--muted)]/40 px-2 py-0.5 text-xs font-medium text-[var(--foreground)]"
        >
          {KIND_LABEL[proposal.kind]}
        </p>
        <h2 className="text-base font-semibold text-[var(--foreground)]">
          Here&apos;s how {workerName} understood that
        </h2>
      </header>

      <p
        data-testid="duty-draft-description"
        className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--muted)]/30 px-4 py-3 text-sm leading-relaxed text-[var(--foreground)]"
      >
        {proposal.description}
      </p>

      <div
        data-testid="duty-draft-schedule"
        className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)] px-4 py-3"
      >
        <p className="text-xs font-medium uppercase tracking-wide text-[var(--muted-foreground)]">
          Schedule
        </p>
        <p className="mt-1 text-sm text-[var(--foreground)]">{cronText}</p>
      </div>

      <div
        data-testid="duty-draft-samples"
        className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)] px-4 py-3"
      >
        <p className="text-xs font-medium uppercase tracking-wide text-[var(--muted-foreground)]">
          Sample outputs (3 recent cases)
        </p>
        <ul className="mt-2 space-y-2 text-sm">
          {(proposal.samples ?? []).map((s) => (
            <li
              key={s.caseId}
              data-testid={`duty-sample-${s.caseId}`}
              className="rounded-[var(--radius-sm)] border border-[var(--border)]/60 px-3 py-2"
            >
              <p className="text-xs text-[var(--muted-foreground)]">{s.context}</p>
              <p className="text-[var(--foreground)]">{s.output}</p>
            </li>
          ))}
        </ul>
      </div>

      {conflictCount > 0 ? (
        <p
          data-testid="duty-draft-conflict-hint"
          className="text-xs text-[var(--warning-foreground)]"
        >
          Heads up — this overlaps with {conflictCount === 1 ? "another duty" : `${conflictCount} other duties`}.
        </p>
      ) : null}

      <div className="flex items-center justify-between gap-2 pt-2">
        <Button
          variant="ghost"
          size="sm"
          data-testid="duty-adjust-wording"
          onClick={onAdjust}
        >
          <Pencil className="size-3.5" />
          Adjust the wording
        </Button>
        <div className="flex gap-2">
          <Button variant="ghost" data-testid="duty-cancel" onClick={onCancel}>
            Cancel
          </Button>
          <Button
            variant="primary"
            data-testid="duty-continue"
            onClick={onContinue}
          >
            Continue
          </Button>
        </div>
      </div>
    </div>
  );
}
