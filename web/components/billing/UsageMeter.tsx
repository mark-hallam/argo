"use client";

/**
 * `UsageMeter` — the plan-identity header on the `/billing` page.
 *
 * Renders the active tier name and the next reset date, straight from
 * `GET /api/v1/tenant/usage`. NO THEATER: every value comes from the
 * live endpoint.
 *
 * **Why no "Actions this month" meter here (de-dup, shakedown re-test
 * N2).** This component used to render its own
 * "Actions this month {used}/{limit}" row + progress bar against the
 * tier's `max_actions_per_month`. `MeterPanel` (DEC-053, #2871) renders
 * the SAME period-to-date `used_actions` count against the bundled
 * `included_actions` allowance, plus overage + cap — so the page showed
 * the metered count twice ("plan 0/100" + "metered 0/Unlimited"), which
 * read as broken duplication. The canonical "Actions this month" meter
 * now lives ONLY in `MeterPanel`; this header carries the plan name +
 * reset date so neither datum is lost. The tier's headline allowance
 * ("100 actions / month") is still shown on the Plans cards below.
 */

import type { UsageResponse } from "@/lib/api/usage";

export function UsageMeter({ usage }: { usage: UsageResponse }) {
  return (
    <div>
      <p className="text-sm uppercase tracking-wide text-[var(--muted-foreground)]">
        Current plan
      </p>
      <p className="mt-1 text-2xl font-semibold tracking-tight">
        {usage.display_name}
      </p>
      <p className="mt-2 text-sm text-[var(--muted-foreground)]">
        Resets {new Date(usage.resets_at).toLocaleDateString()}.
      </p>
    </div>
  );
}
