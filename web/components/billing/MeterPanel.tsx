"use client";

/**
 * `MeterPanel` — the honest included / used / overage / cap meter on the
 * `/billing` page, wired to `GET /api/v1/billing/meter` (DEC-053, #2871).
 *
 * Shows the bundled monthly allowance (`included`), the live period-to-date
 * action count (`used`) against it, the metered overage units, a *labelled
 * estimate* of overage dollars, and the effective hard monthly spend cap.
 *
 * Shakedown F11 fix: before this, the billing usage panel could sit on a
 * perpetual "Loading usage…" spinner because nothing wired the meter API.
 * This component owns its fetch with a bounded timeout (`METER_TIMEOUT_MS`)
 * + `retry: 1`, so it always settles to data, an honest empty state, or a
 * plain error — never an infinite spinner. NO THEATER (DEC-029): every
 * number comes from the live endpoint; the estimate is shown as an
 * estimate, never as a charged amount.
 */

import { useQuery } from "@tanstack/react-query";
import { ProgressBar } from "@/components/ui/charts";
import {
  fetchBillingMeter,
  meterUsagePercent,
  type MeterResponse,
} from "@/lib/api/meter";

function formatUsd(usd: number): string {
  if (!Number.isFinite(usd) || usd <= 0) return "$0.00";
  const places = usd < 1 ? 4 : 2;
  return `$${usd.toFixed(places)}`;
}

export function MeterPanel() {
  const { data, isLoading, error } = useQuery<MeterResponse>({
    queryKey: ["billing", "meter"],
    queryFn: fetchBillingMeter,
    // Bounded: one quick retry, then settle to the error state. With the
    // 8s per-attempt fetch timeout this caps the loading window at ~16s —
    // never an infinite spinner (shakedown F11).
    retry: 1,
  });

  if (isLoading) {
    return (
      <div data-testid="meter-loading" className="mt-2 text-sm">
        Loading meter…
      </div>
    );
  }

  if (error || !data) {
    return (
      <div
        data-testid="meter-error"
        className="mt-2 text-sm text-[var(--muted-foreground)]"
      >
        Usage meter is unavailable right now. Your plan and spend above are
        still accurate; the action meter will return shortly.
      </div>
    );
  }

  const hasAllowance = data.included_actions > 0;
  const pct = meterUsagePercent(data.used_actions, data.included_actions);
  const overage = data.overage_units > 0;

  return (
    <div data-testid="meter-panel" className="mt-2">
      <div className="flex items-baseline justify-between">
        <span className="text-sm font-medium">Actions this month</span>
        <span className="text-sm text-[var(--muted-foreground)] tabular-nums">
          {data.used_actions.toLocaleString()} /{" "}
          {hasAllowance
            ? data.included_actions.toLocaleString()
            : "Unlimited"}{" "}
          included
        </span>
      </div>

      {/*
        Self-explanatory label (shakedown re-test N2): this meter counts
        BILLABLE METERED ACTIONS (one per session / forge run / channel
        message that crosses the gateway quota seam), NOT raw provider
        spend. A run can accrue provider $ — shown in the Spend panel
        below — without incrementing a metered action (e.g. an internal
        duty / probation run that never crosses an HTTP quota seam), so a
        0 here next to nonzero spend is honest, not broken. Labelling the
        unit is the truth-bounded fix; the counter is never fabricated up
        to match spend.
      */}
      <p
        data-testid="meter-unit-note"
        className="mt-1 text-xs text-[var(--muted-foreground)]"
      >
        Counts billable metered actions. Provider spend is metered
        separately (see Spend below).
      </p>

      {hasAllowance ? (
        <div className="mt-2">
          <ProgressBar
            value={pct}
            thresholds={{ warning: 80, destructive: 95 }}
            ariaLabel="Included action allowance used"
          />
        </div>
      ) : (
        <p className="mt-2 text-sm text-[var(--muted-foreground)]">
          No metered allowance on this plan.
        </p>
      )}

      <dl className="mt-4 grid grid-cols-2 gap-x-4 gap-y-2 text-sm">
        <dt className="text-[var(--muted-foreground)]">Overage</dt>
        <dd className="text-right tabular-nums">
          {overage
            ? `${data.overage_units.toLocaleString()} actions`
            : "None"}
        </dd>

        {overage && (
          <>
            <dt className="text-[var(--muted-foreground)]">
              Estimated overage
            </dt>
            <dd className="text-right tabular-nums">
              {formatUsd(data.estimated_overage_usd)}{" "}
              <span className="text-[var(--muted-foreground)]">est.</span>
            </dd>
          </>
        )}

        <dt className="text-[var(--muted-foreground)]">Monthly cap</dt>
        <dd className="text-right tabular-nums">
          {data.cap_usd === null
            ? "No cap"
            : `${formatUsd(data.cap_usd)}${data.cap_is_default ? " (default)" : ""}`}
        </dd>
      </dl>
    </div>
  );
}
