"use client";

/**
 * `/billing/subscribe?plan=pro` — landing for a deep-linked upgrade
 * action. Reads the plan slug from the URL (accepts `?plan=` or the
 * marketing-friendly `?tier=` alias), resolves the matching Stripe
 * price id from `NEXT_PUBLIC_STRIPE_PRICE_<TIER>`, kicks off Checkout,
 * and redirects.
 *
 * Used by marketing CTAs and inbound product emails that want to drop
 * the customer straight into Checkout without an extra click.
 */

import { Suspense, useEffect, useState, type ReactNode } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { Button } from "@/components/ui/Button";
import { useCheckout, redirectToCheckout } from "@/lib/billing/use-checkout";

// Static LITERAL env reads so Next.js inlines the NEXT_PUBLIC_* values into the
// client bundle at build time. A DYNAMIC read (process.env[varName]) is NOT
// inlined and resolves to undefined in the browser, which made every upgrade
// fail "No Stripe price configured" even when the build-arg was set. See
// app/pricing/page.tsx which reads the same var statically (and works).
const PRICE_BY_PLAN: Record<string, string | undefined> = {
  pro: process.env.NEXT_PUBLIC_STRIPE_PRICE_PRO,
  enterprise: process.env.NEXT_PUBLIC_STRIPE_PRICE_ENTERPRISE,
};

export default function BillingSubscribePage() {
  return (
    <Suspense fallback={<SubscribeFallback />}>
      <SubscribeContent />
    </Suspense>
  );
}

function SubscribeShell({
  title,
  children,
}: {
  title: string;
  children: ReactNode;
}) {
  return (
    <div className="mx-auto max-w-md px-4 sm:px-6 lg:px-8 py-12 space-y-4">
      <h1 className="text-xl font-semibold tracking-tight">{title}</h1>
      {children}
    </div>
  );
}

function SubscribeFallback() {
  return (
    <SubscribeShell title="Subscribe">
      <p className="text-sm">Preparing your upgrade…</p>
    </SubscribeShell>
  );
}

function SubscribeContent() {
  const router = useRouter();
  const params = useSearchParams();
  const plan = params?.get("plan") ?? params?.get("tier") ?? "pro";
  const priceId = PRICE_BY_PLAN[plan];
  const checkout = useCheckout();
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  useEffect(() => {
    if (!priceId) {
      setErrorMessage(
        `No Stripe price configured for plan "${plan}". Set NEXT_PUBLIC_STRIPE_PRICE_${plan.toUpperCase()} at build time.`,
      );
      return;
    }
    redirectToCheckout(checkout.mutateAsync, { priceId }).catch((err) => {
      setErrorMessage(err instanceof Error ? err.message : String(err));
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [priceId]);

  if (errorMessage) {
    return (
      <SubscribeShell title="Couldn’t start checkout">
        <p className="text-sm text-[var(--destructive)]">{errorMessage}</p>
        <Button
          variant="secondary"
          onClick={() => router.push("/billing")}
          type="button"
        >
          Back to billing
        </Button>
      </SubscribeShell>
    );
  }
  return (
    <SubscribeShell title="Subscribe">
      <p className="text-sm">Redirecting to Stripe Checkout…</p>
    </SubscribeShell>
  );
}
