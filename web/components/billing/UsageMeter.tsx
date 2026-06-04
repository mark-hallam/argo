"use client";

/**
 * `UsageMeter` — the visual usage meter on the `/billing` page.
 *
 * Renders the active tier and a progress bar of period-to-date actions
 * consumed against the tier's monthly cap, straight from
 * `GET /api/v1/tenant/usage`. NO THEATER: every number comes from the
 * live endpoint. Zero usage renders an honest empty bar (never a
 * fabricated count); the unlimited tier (`limit === 0`) renders the
 * count with no bar, because there is no percentage of an unlimited
 * cap.
 *
 * The live usage endpoint does not expose a worker count or
 * `max_workers`, so this meter deliberately does not render a worker
 * gauge — fabricating those numbers would violate the truth boundary.
 * If/when the endpoint grows those fields, extend `UsageResponse` and
 * add a second gauge here.
 */

import { ProgressBar } from "@/components/ui/charts";
import { usagePercent, type UsageResponse } from "@/lib/api/usage";

export function UsageMeter({ usage }: { usage: UsageResponse }) {
  const unlimited = usage.limit === 0;
  const pct = usagePercent(usage.used, usage.limit);
  const limitLabel = unlimited ? "Unlimited" : usage.limit.toLocaleString();

  return (
    <div>
      <p className="text-sm uppercase tracking-wide text-[var(--muted-foreground)]">
        Current plan
      </p>
      <p className="mt-1 text-2xl font-semibold tracking-tight">
        {usage.display_name}
      </p>

      <div className="mt-4">
        <div className="flex items-baseline justify-between">
          <span className="text-sm font-medium">Actions this month</span>
          <span className="text-sm text-[var(--muted-foreground)]">
            {usage.used.toLocaleString()} / {limitLabel}
          </span>
        </div>

        {unlimited ? (
          <p className="mt-2 text-sm text-[var(--muted-foreground)]">
            Unlimited actions on this plan.
          </p>
        ) : (
          <div className="mt-2">
            <ProgressBar
              value={pct}
              thresholds={{ warning: 80, destructive: 95 }}
              ariaLabel="Monthly action usage"
            />
          </div>
        )}
      </div>

      <p className="mt-3 text-sm text-[var(--muted-foreground)]">
        Used {usage.used.toLocaleString()} of {limitLabel} actions this month.
        {" "}
        Resets {new Date(usage.resets_at).toLocaleDateString()}.
      </p>
    </div>
  );
}
