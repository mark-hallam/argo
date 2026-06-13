"use client";

/**
 * `/settings/billing` — thin plan summary inside Settings.
 *
 * The active plan label is read from the SAME canonical source the
 * `/billing` page uses: `GET /api/v1/tenant/usage` via
 * `fetchTenantUsage()`, rendering the endpoint's `display_name`. This
 * panel previously hardcoded `<strong>Pro</strong>`, which disagreed
 * with `/billing` for any tenant not actually on Pro (soak final
 * scorecard 2026-06-13: Settings said "Pro" while `/billing` said
 * "Free" for the same demo tenant). One source of truth — both
 * surfaces now show the same label for a given tenant.
 */

import Link from "next/link";
import { useEffect, useState } from "react";
import { fetchTenantUsage } from "@/lib/api/usage";

export default function BillingSettingsPage() {
  // `null` = not yet loaded / unavailable → render an em dash rather
  // than a fabricated tier (NO THEATER).
  const [planLabel, setPlanLabel] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    fetchTenantUsage()
      .then((usage) => {
        if (active) setPlanLabel(usage.display_name);
      })
      .catch(() => {
        if (active) setPlanLabel(null);
      });
    return () => {
      active = false;
    };
  }, []);

  return (
    <section className="settings-panel" data-testid="settings-billing-panel">
      <div className="settings-panel-head">
        <div>
          <p className="settings-kicker">Billing</p>
          <h2>Plan and usage</h2>
          <p>
            Billing remains tied to the account-level billing surface while
            settings keeps the workspace context visible.
          </p>
        </div>
        <Link className="settings-inline-action" href="/billing">
          Open billing
        </Link>
      </div>

      <div className="settings-fields">
        <div className="settings-field">
          <div>
            <span className="settings-field-label">Plan</span>
            <p>Controls connectors, notification channels, and worker concurrency.</p>
          </div>
          <strong data-testid="settings-billing-plan">{planLabel ?? "—"}</strong>
        </div>
        <div className="settings-field">
          <div>
            <span className="settings-field-label">Invoice owner</span>
            <p>Receives subscription receipts and usage-threshold alerts.</p>
          </div>
          <strong>Workspace owner</strong>
        </div>
        <div className="settings-field">
          <div>
            <span className="settings-field-label">Usage window</span>
            <p>Worker runs and connector calls reset on the plan anniversary.</p>
          </div>
          <code>Monthly</code>
        </div>
      </div>
    </section>
  );
}
