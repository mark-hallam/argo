"use client";

import { useEffect, useState } from "react";
import { cn } from "@/lib/utils";

/**
 * Time-to-expire bar on each approval card.
 *  - Green   when at least 4h remain
 *  - Amber   when at least 1h remains
 *  - Red     when less than 1h remains (over-SLA when remaining <= 0)
 *
 * The fill width visualises the share of the original SLA budget that
 * has *already burned* — wider = more urgent. The component recomputes
 * remaining time every 30s so the bar drifts toward red as time passes;
 * SSR/CSR mismatch is avoided by deferring the first computation to the
 * client (mirrors the `RelativeTime` pattern in `ApprovalCard.tsx`).
 *
 * Spec-side defaults (from the locked E-ops-dashboard design):
 *   - 24h is the standard SLA window for severity P1/P2.
 *   - P0 over-SLA renders as 95% fill, red.
 */

export interface SLABarProps {
  /** ISO timestamp when the approval times out. */
  expiresAt: string;
  /** Original SLA window in milliseconds; defaults to 24h. */
  windowMs?: number;
  className?: string;
}

const DEFAULT_WINDOW_MS = 24 * 60 * 60 * 1000;
const HOUR = 60 * 60 * 1000;

function tone(remainingMs: number): "ok" | "warn" | "crit" {
  if (remainingMs >= 4 * HOUR) return "ok";
  if (remainingMs >= 1 * HOUR) return "warn";
  return "crit";
}

function formatRemaining(ms: number): string {
  if (ms <= 0) {
    const overMs = -ms;
    if (overMs < HOUR) return `over ${Math.round(overMs / 60_000)}m`;
    return `over ${Math.floor(overMs / HOUR)}h`;
  }
  if (ms < HOUR) return `${Math.round(ms / 60_000)}m left`;
  const hours = Math.floor(ms / HOUR);
  const minutes = Math.round((ms - hours * HOUR) / 60_000);
  if (minutes === 0) return `${hours}h left`;
  return `${hours}h ${minutes}m left`;
}

export function SLABar({
  expiresAt,
  windowMs = DEFAULT_WINDOW_MS,
  className,
}: SLABarProps) {
  // Defer to client tick to avoid SSR/CSR mismatch on `Date.now()`.
  const [now, setNow] = useState<number | null>(null);
  useEffect(() => {
    setNow(Date.now());
    const id = setInterval(() => setNow(Date.now()), 30_000);
    return () => clearInterval(id);
  }, []);

  if (now == null) {
    // SSR placeholder — invisible track, no JS-dependent state.
    return (
      <div className={cn("space-y-1", className)} data-testid="sla-bar-pending">
        <div className="h-1 rounded-full bg-[var(--muted)]" aria-hidden />
        <div className="text-[11px] font-mono text-[var(--muted-foreground)]">
          {/* Static label so the layout doesn't jump on hydration. */}
          &nbsp;
        </div>
      </div>
    );
  }

  const target = new Date(expiresAt).getTime();
  const remaining = target - now;
  const t = tone(remaining);
  const burnedPct = Math.max(
    0,
    Math.min(100, ((windowMs - Math.max(0, remaining)) / windowMs) * 100),
  );
  const fillPct = remaining <= 0 ? 95 : burnedPct;

  const fillCls =
    t === "crit"
      ? "bg-[var(--destructive)]"
      : t === "warn"
        ? "bg-[var(--warning)]"
        : "bg-[var(--success)]";

  const labelCls =
    t === "crit"
      ? "text-[var(--destructive)] font-semibold"
      : "text-[var(--muted-foreground)]";

  return (
    <div className={cn("space-y-1", className)} data-testid="sla-bar" data-tone={t}>
      <div
        className="h-1 overflow-hidden rounded-full bg-[var(--muted)]"
        role="progressbar"
        aria-label="time remaining before SLA breach"
        aria-valuenow={Math.round(remaining / 60_000)}
        aria-valuemin={0}
        aria-valuemax={Math.round(windowMs / 60_000)}
      >
        <div
          className={cn("h-full rounded-full", fillCls)}
          style={{ width: `${fillPct}%` }}
        />
      </div>
      <div className={cn("text-[11px] font-mono leading-none", labelCls)}>
        {remaining <= 0 ? "⚠ " : null}
        {formatRemaining(remaining)}
      </div>
    </div>
  );
}
