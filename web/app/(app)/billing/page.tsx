"use client";

/**
 * `/billing` — tenant billing overview.
 *
 * Surfaces the active tier (Free / Pro / Enterprise), period-to-date
 * usage, the three upgrade CTAs, and the Manage button that opens a
 * Stripe Customer Portal session. Data fetched from
 * `GET /api/v1/tenant/usage` (claude-pricing-plans-config).
 */

import { useQuery } from "@tanstack/react-query";
import { Suspense } from "react";
import { apiGet } from "@/lib/api/client";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { useCheckout, redirectToCheckout } from "@/lib/billing/use-checkout";
import { usePortal, redirectToPortal } from "@/lib/billing/use-portal";

interface UsageResponse {
  tier: "free" | "pro" | "enterprise";
  display_name: string;
  used: number;
  limit: number;
  remaining: number | null;
  resets_at: string;
}

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

const PLANS: PlanOffer[] = [
  {
    tier: "free",
    title: "Free",
    price: "$0",
    blurb: "Try Skycot with the mock provider.",
    features: ["1 worker", "100 actions / month", "Community support"],
    priceEnv: "",
    cta: "Current tier",
  },
  {
    tier: "pro",
    title: "Pro",
    price: "$29 / month",
    blurb: "Real provider keys, multi-channel, marketplace skills.",
    features: ["5 workers", "5,000 actions / month", "Email support"],
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
  const { data, isLoading, error } = useQuery<UsageResponse>({
    queryKey: ["billing", "usage"],
    queryFn: () => apiGet<UsageResponse>("/api/v1/tenant/usage"),
  });
  const checkout = useCheckout();
  const portal = usePortal();

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
        {data && <CurrentTierSummary usage={data} />}
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

      <section>
        <h2 className="text-lg font-semibold tracking-tight">Plans</h2>
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

function CurrentTierSummary({ usage }: { usage: UsageResponse }) {
  const limitLabel = usage.limit === 0 ? "Unlimited" : usage.limit.toLocaleString();
  return (
    <div>
      <p className="text-sm uppercase tracking-wide text-[var(--muted-foreground)]">
        Current plan
      </p>
      <p className="mt-1 text-2xl font-semibold tracking-tight">
        {usage.display_name}
      </p>
      <p className="mt-2 text-sm text-[var(--muted-foreground)]">
        Used {usage.used.toLocaleString()} of {limitLabel} actions this month.
        Resets {new Date(usage.resets_at).toLocaleDateString()}.
      </p>
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
  const priceId = plan.priceEnv ? process.env[plan.priceEnv] : "";
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
