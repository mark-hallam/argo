"use client";

/**
 * `/billing` — tenant billing overview. **Canonical billing surface.**
 *
 * Surfaces the active tier (Free / Pro / Enterprise), period-to-date
 * usage, the three upgrade CTAs, and the Manage button that opens a
 * Stripe Customer Portal session. Data fetched from
 * `GET /api/v1/tenant/usage` (claude-pricing-plans-config).
 *
 * Intentionally not in the primary sidebar nav — reachable from:
 *   - `/settings/billing` "Open billing" button (the settings panel
 *     is a thin summary that links here for the real plan/checkout
 *     flow).
 *   - `/billing/subscribe` post-checkout return.
 *   - The ⌘K command palette (`page:billing`).
 *
 * Audit history: the orphan-route audit
 * (whole-site-mvp-claims-coverage F18) flagged this page as an
 * "orphan duplicate" of `/settings/billing`. That premise was
 * incorrect — `/settings/billing/page.tsx:15` links *to* this page,
 * not the other way around. Documenting the role per
 * fix-spa-polish-help-retro-billing-orphan-pages so the next audit
 * doesn't re-flag it.
 */

import { useQuery } from "@tanstack/react-query";
import { Suspense, useEffect, useState } from "react";
import { useSearchParams } from "next/navigation";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { useCheckout, redirectToCheckout } from "@/lib/billing/use-checkout";
import { usePortal, redirectToPortal } from "@/lib/billing/use-portal";
import { fetchTenantUsage, type UsageResponse } from "@/lib/api/usage";
import { FREE, PRO, usd, actions } from "@/lib/pricing/tiers";
import { UsageMeter } from "@/components/billing/UsageMeter";
import { MeterPanel } from "@/components/billing/MeterPanel";
import { SpendPanel } from "@/components/billing/SpendPanel";

interface PlanOffer {
  tier: "free" | "pro" | "enterprise";
  title: string;
  price: string;
  blurb: string;
  features: string[];
  /** Stripe price id — left empty for the Free tier (no Checkout). */
  priceEnv: string;
  cta: string;
}

// Plan numbers (price, worker + action allowances) are read from the
// canonical pricing source of truth (docs/skycot/pricing/v1-tiers.toml →
// tiers.data.json) via `@/lib/pricing/tiers`, so this page can never drift
// from /pricing again. Only the billing-specific fields (Stripe priceEnv,
// CTA label) are local.
const PLANS: PlanOffer[] = [
  {
    tier: "free",
    title: "Free",
    price: usd(FREE.price_monthly_usd),
    blurb: "Real responses on Skycot's keys, capped at the free limit.",
    features: [
      `${FREE.max_workers} teammate`,
      `${actions(FREE.max_actions_per_month)} actions / month`,
      "Community support",
    ],
    priceEnv: "",
    cta: "Current tier",
  },
  {
    tier: "pro",
    title: "Pro",
    price: `${usd(PRO.price_monthly_usd)} / teammate / month`,
    blurb: "Real provider keys, multi-channel, marketplace skills.",
    features: [
      `Up to ${PRO.max_workers} teammates`,
      `${actions(PRO.included_actions)} actions / teammate included`,
      "Email support",
    ],
    priceEnv: "NEXT_PUBLIC_STRIPE_PRICE_PRO",
    cta: "Upgrade to Pro",
  },
  {
    tier: "enterprise",
    title: "Enterprise",
    price: "Custom",
    blurb: "SSO, audit export, custom verticals, priority queue.",
    features: ["Unlimited workers", "Unlimited actions", "Dedicated support"],
    priceEnv: "NEXT_PUBLIC_STRIPE_PRICE_ENTERPRISE",
    cta: "Contact sales",
  },
];

// Static LITERAL env reads (inlined by Next at build time) keyed by tier. A
// dynamic read — process.env[plan.priceEnv] — is NOT inlined into the client
// bundle and resolves to undefined, which disabled every upgrade button. Keyed
// by tier so it stays in lockstep with PLANS.
const PRICE_BY_TIER: Record<string, string | undefined> = {
  pro: process.env.NEXT_PUBLIC_STRIPE_PRICE_PRO,
  enterprise: process.env.NEXT_PUBLIC_STRIPE_PRICE_ENTERPRISE,
};

/**
 * Bounded backoff for the post-checkout tier auto-refresh poll.
 *
 * After Stripe Checkout redirects the buyer to `/billing?upgrade=ok`,
 * the tier-flip webhook (`POST /api/v1/billing/webhook`) is async and
 * usually lands a beat after the redirect. Rather than stranding a
 * just-paid customer on their OLD plan until they manually refresh, we
 * re-poll the SAME usage query (no parallel fetch) on this schedule
 * until the tier flips off `free`, then stop. Five attempts, ~1.5s →
 * 6s capped, ≤ ~20s total. If the cap is hit without a flip we fall
 * back to a calm "still processing" note (the webhook may just be slow,
 * not broken). NO THEATER (DEC-029): we never render "Pro active"
 * before the live tier actually flips.
 */
const POST_CHECKOUT_POLL_DELAYS_MS = [1500, 2500, 4000, 6000, 6000];

export default function BillingPage() {
  return (
    <Suspense fallback={<BillingFallback />}>
      <BillingContent />
    </Suspense>
  );
}

function BillingFallback() {
  return (
    <div className="mx-auto max-w-5xl px-4 sm:px-6 lg:px-8 py-8">
      <h1 className="text-xl font-semibold tracking-tight">Billing</h1>
      <p className="mt-2 text-sm text-[var(--muted-foreground)]">Loading…</p>
    </div>
  );
}

function BillingContent() {
  const { data, isLoading, error, refetch } = useQuery<UsageResponse>({
    queryKey: ["billing", "usage"],
    queryFn: fetchTenantUsage,
    // Bounded: one quick retry then surface the error state, so the plan
    // header never sits on a perpetual "Loading usage…" (shakedown F11).
    retry: 1,
  });
  const checkout = useCheckout();
  const portal = usePortal();

  // Post-checkout tier auto-refresh. Stripe returns the buyer to
  // `/billing?upgrade=ok`; the tier-flip webhook is async, so the tier
  // we just fetched is usually still the pre-upgrade `free`. While that
  // is true we re-poll the EXISTING usage query (same queryKey /
  // fetchTenantUsage — no parallel fetch) with bounded backoff until it
  // flips, then stop.
  const params = useSearchParams();
  const justUpgraded = params?.get("upgrade") === "ok";
  const tierStillFree = !!data && data.tier === "free";
  const [pollAttempts, setPollAttempts] = useState(0);
  const isPolling =
    justUpgraded &&
    tierStillFree &&
    pollAttempts < POST_CHECKOUT_POLL_DELAYS_MS.length;
  const pollExhausted =
    justUpgraded &&
    tierStillFree &&
    pollAttempts >= POST_CHECKOUT_POLL_DELAYS_MS.length;

  useEffect(() => {
    if (!isPolling) return;
    const delay = POST_CHECKOUT_POLL_DELAYS_MS[pollAttempts];
    const timer = setTimeout(() => {
      void refetch();
      setPollAttempts((n) => n + 1);
    }, delay);
    return () => clearTimeout(timer);
  }, [isPolling, pollAttempts, refetch]);

  return (
    <div className="mx-auto max-w-5xl px-4 sm:px-6 lg:px-8 py-8 space-y-6">
      <header>
        <h1 className="text-xl font-semibold tracking-tight">Billing</h1>
        <p className="mt-1 text-sm text-[var(--muted-foreground)]">
          Manage your subscription, payment methods, and invoices.
        </p>
      </header>

      <Card className="p-6">
        {isLoading && <p className="text-sm">Loading usage…</p>}
        {error && (
          <p className="text-sm text-[var(--destructive)]">
            Failed to load usage: {(error as Error).message}
          </p>
        )}
        {data && <UsageMeter usage={data} />}
        <MeterPanel />
        <div className="mt-4 flex gap-2">
          <Button
            type="button"
            variant="secondary"
            disabled={portal.isPending || data?.tier === "free"}
            onClick={() => {
              void redirectToPortal(portal.mutateAsync, {});
            }}
          >
            {portal.isPending ? "Opening…" : "Manage subscription"}
          </Button>
        </div>
      </Card>

      <SpendPanel />

      <section>
        <h2 className="text-lg font-semibold tracking-tight">Plans</h2>
        {isPolling && (
          <p
            role="status"
            className="mt-2 text-sm text-[var(--muted-foreground)]"
          >
            Processing your upgrade… your new plan will appear here in a
            moment.
          </p>
        )}
        {pollExhausted && (
          <p
            role="status"
            className="mt-2 text-sm text-[var(--muted-foreground)]"
          >
            Still processing. Your upgrade is on its way. Refresh in a
            moment if your new plan isn’t shown yet.
          </p>
        )}
        <div className="mt-3 grid gap-4 sm:grid-cols-3">
          {PLANS.map((plan) => (
            <PlanCard
              key={plan.tier}
              plan={plan}
              currentTier={data?.tier}
              busy={checkout.isPending}
              onUpgrade={(priceId) => {
                void redirectToCheckout(checkout.mutateAsync, { priceId });
              }}
            />
          ))}
        </div>
        {checkout.error && (
          <p className="mt-2 text-sm text-[var(--destructive)]">
            Checkout failed: {(checkout.error as Error).message}
          </p>
        )}
      </section>
    </div>
  );
}

function PlanCard({
  plan,
  currentTier,
  busy,
  onUpgrade,
}: {
  plan: PlanOffer;
  currentTier?: UsageResponse["tier"];
  busy: boolean;
  onUpgrade: (priceId: string) => void;
}) {
  const isCurrent = currentTier === plan.tier;
  const priceId = PRICE_BY_TIER[plan.tier] ?? "";
  const disabled = isCurrent || busy || !priceId;
  return (
    <Card className="p-4 flex flex-col gap-3" selected={isCurrent}>
      <header>
        <h3 className="text-base font-semibold tracking-tight">{plan.title}</h3>
        <p className="text-sm text-[var(--muted-foreground)]">{plan.price}</p>
      </header>
      <p className="text-sm">{plan.blurb}</p>
      <ul className="text-sm space-y-1">
        {plan.features.map((f) => (
          <li key={f}>• {f}</li>
        ))}
      </ul>
      <div className="mt-auto pt-2">
        {plan.tier === "free" || !priceId ? (
          <Button variant="ghost" disabled>
            {isCurrent ? "Current tier" : plan.cta}
          </Button>
        ) : (
          <Button
            type="button"
            disabled={disabled}
            onClick={() => onUpgrade(priceId)}
          >
            {isCurrent ? "Current tier" : plan.cta}
          </Button>
        )}
      </div>
    </Card>
  );
}
