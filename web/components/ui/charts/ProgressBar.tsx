import { cn } from "@/lib/utils";
import { thresholdTone } from "./utils";

/**
 * Thin horizontal progress bar. The fill colour is derived from
 * `value` against the supplied `thresholds`: below `warning` →
 * muted, `[warning, destructive)` → warning, `>= destructive` →
 * destructive. With no thresholds the bar is always muted.
 *
 * Values are clamped to [0, 100]. Non-finite values render at 0.
 */

export interface ProgressBarProps {
  /** 0–100, inclusive. Out-of-range values clamp; NaN → 0. */
  value: number;
  thresholds?: { warning?: number; destructive?: number };
  /** Defaults to 6. */
  height?: number;
  ariaLabel?: string;
  className?: string;
}

function fillClassFor(tone: "muted" | "warning" | "destructive"): string {
  switch (tone) {
    case "destructive":
      return "bg-[var(--destructive)]";
    case "warning":
      return "bg-[var(--warning)]";
    case "muted":
    default:
      return "bg-[var(--muted-foreground)]";
  }
}

export function ProgressBar({
  value,
  thresholds = {},
  height = 6,
  ariaLabel,
  className,
}: ProgressBarProps) {
  const safe = Number.isFinite(value) ? Math.max(0, Math.min(100, value)) : 0;
  const tone = thresholdTone(safe, thresholds);
  const label = ariaLabel ?? "Progress";
  return (
    <div
      role="progressbar"
      aria-label={label}
      aria-valuenow={Math.round(safe)}
      aria-valuemin={0}
      aria-valuemax={100}
      data-testid="progress-bar"
      data-tone={tone}
      className={cn(
        "relative w-full overflow-hidden rounded-full bg-[var(--muted)]",
        className,
      )}
      style={{ height }}
    >
      <span className="sr-only">
        {label}: {Math.round(safe)} percent
      </span>
      <div
        data-testid="progress-bar-fill"
        className={cn(
          "h-full transition-[width] duration-[var(--duration-base)]",
          fillClassFor(tone),
        )}
        style={{ width: `${safe}%` }}
      />
    </div>
  );
}
