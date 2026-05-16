"use client";

/**
 * `useCheckout` — open a Stripe Checkout session for the active tenant.
 *
 * `mutateAsync({ priceId })` POSTs the chosen price id to the gateway
 * (which proxies to `POST /v1/checkout/sessions` with the tenant id
 * stamped into Checkout metadata). On success the SPA redirects to the
 * hosted Checkout URL Stripe returned.
 */

import { useMutation } from "@tanstack/react-query";
import { apiPost } from "@/lib/api/client";

/** Backend response from `POST /api/v1/billing/checkout`. */
export interface CheckoutResponse {
  id: string;
  url: string;
}

/** Caller-supplied input. `successUrl` / `cancelUrl` default to the SPA's billing page. */
export interface CheckoutInput {
  priceId: string;
  successUrl?: string;
  cancelUrl?: string;
  customerEmail?: string;
}

function origin(): string {
  if (typeof window === "undefined") return "https://app.skycot.com";
  return window.location.origin;
}

export function useCheckout() {
  return useMutation<CheckoutResponse, Error, CheckoutInput>({
    mutationFn: async ({ priceId, successUrl, cancelUrl, customerEmail }) => {
      const body = {
        price_id: priceId,
        success_url: successUrl ?? `${origin()}/billing?upgrade=ok`,
        cancel_url: cancelUrl ?? `${origin()}/billing?upgrade=cancel`,
        customer_email: customerEmail,
      };
      return apiPost<CheckoutResponse, typeof body>(
        "/api/v1/billing/checkout",
        body,
      );
    },
  });
}

/** Helper — perform `mutateAsync` and follow the returned URL. */
export async function redirectToCheckout(
  mutateAsync: (input: CheckoutInput) => Promise<CheckoutResponse>,
  input: CheckoutInput,
): Promise<void> {
  const response = await mutateAsync(input);
  if (typeof window !== "undefined") {
    window.location.assign(response.url);
  }
}
