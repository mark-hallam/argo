import { Loader2 } from "lucide-react";

/**
 * Loading-state progress block — spinner + "Replaying… step n of m"
 * label + linear progress bar. v1 server doesn't stream per-step
 * progress (that's the sibling backend brief); the page advances a
 * simulated 4-stage clock so the surface doesn't feel frozen.
 *
 * Stages are deliberately abstract — server-side, the stages will
 * be: rehydrate-memory → replay-skill → score-against-policy →
 * compare-output. The labels match.
 */

export const REPLAY_STAGES = [
  "Rehydrating memory snapshot",
  "Replaying skill execution",
  "Evaluating against current policy",
  "Comparing output to original",
] as const;

export type ReplayStage = (typeof REPLAY_STAGES)[number];

export interface ReplayProgressProps {
  /** Current step, 1-indexed. */
  current: number;
  /** Total number of steps. */
  total: number;
}

export function ReplayProgress({ current, total }: ReplayProgressProps) {
  const clamped = Math.max(1, Math.min(current, total));
  const pct = Math.round((clamped / total) * 100);
  const label = REPLAY_STAGES[Math.min(clamped - 1, REPLAY_STAGES.length - 1)];
  return (
    <div
      data-testid="replay-progress"
      role="status"
      aria-live="polite"
      className="mx-auto max-w-md rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] px-6 py-8 text-center space-y-3"
    >
      <Loader2 className="mx-auto size-6 animate-spin text-[var(--primary)]" />
      <p
        data-testid="replay-progress-label"
        className="text-sm font-medium text-[var(--foreground)]"
      >
        Replaying… step {clamped} of {total}
      </p>
      <p className="text-xs text-[var(--muted-foreground)]">{label}</p>
      <div
        className="relative h-1.5 w-full overflow-hidden rounded-full bg-[var(--muted)]"
        role="progressbar"
        aria-valuenow={pct}
        aria-valuemin={0}
        aria-valuemax={100}
        data-testid="replay-progress-bar"
      >
        <div
          className="absolute inset-y-0 left-0 bg-[var(--primary)] transition-[width] duration-300 ease-out"
          style={{ width: `${pct}%` }}
        />
      </div>
    </div>
  );
}
