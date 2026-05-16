"use client";

/**
 * `/billing/subscribe?plan=pro` — landing for a deep-linked upgrade
 * action. Reads the plan slug from the URL, resolves the matching
 * Stripe price id from `NEXT_PUBLIC_STRIPE_PRICE_<TIER>`, kicks off
 * Checkout, and redirects.
 *
 * Used by marketing CTAs and inbound product emails that want to drop
 * the customer straight into Checkout without an extra click.
 */

import { Suspense, useEffect, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { Button } from "@/components/ui/Button";
import { useCheckout, redirectToCheckout } from "@/lib/billing/use-checkout";

const PRICE_ENV: Record<string, string> = {
  pro: "NEXT_PUBLIC_STRIPE_PRICE_PRO",
  enterprise: "NEXT_PUBLIC_STRIPE_PRICE_ENTERPRISE",
};

export default function BillingSubscribePage() {
  return (
    <Suspense fallback={<SubscribeFallback />}>
      <SubscribeContent />
    </Suspense>
  );
}

function SubscribeFallback() {
  return (
    <div className="mx-auto max-w-md px-4 sm:px-6 lg:px-8 py-12">
      <p className="text-sm">Preparing your upgrade…</p>
    </div>
  );
}

function SubscribeContent() {
  const router = useRouter();
  const params = useSearchParams();
  const plan = params?.get("plan") ?? "pro";
  const priceEnv = PRICE_ENV[plan];
  const priceId = priceEnv ? process.env[priceEnv] : undefined;
  const checkout = useCheckout();
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  useEffect(() => {
    if (!priceId) {
      setErrorMessage(
        `No Stripe price configured for plan "${plan}". Set ${priceEnv ?? "NEXT_PUBLIC_STRIPE_PRICE_<TIER>"}.`,
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
      <div className="mx-auto max-w-md px-4 sm:px-6 lg:px-8 py-12 space-y-4">
        <h1 className="text-lg font-semibold tracking-tight">Couldn’t start checkout</h1>
        <p className="text-sm text-[var(--destructive)]">{errorMessage}</p>
        <Button
          variant="secondary"
          onClick={() => router.push("/billing")}
          type="button"
        >
          Back to billing
        </Button>
      </div>
    );
  }
  return (
    <div className="mx-auto max-w-md px-4 sm:px-6 lg:px-8 py-12">
      <p className="text-sm">Redirecting to Stripe Checkout…</p>
    </div>
  );
}
