"use client";

/**
 * `usePortal` — open a Stripe Customer Portal session so the customer
 * can manage payment methods, view invoices, switch plans, or cancel.
 */

import { useMutation } from "@tanstack/react-query";
import { apiPost } from "@/lib/api/client";

/** Backend response from `POST /api/v1/billing/portal`. */
export interface PortalResponse {
  id: string;
  url: string;
}

/** Caller-supplied input. `returnUrl` defaults to the SPA's billing page. */
export interface PortalInput {
  returnUrl?: string;
}

function origin(): string {
  if (typeof window === "undefined") return "https://app.skycot.com";
  return window.location.origin;
}

export function usePortal() {
  return useMutation<PortalResponse, Error, PortalInput>({
    mutationFn: async ({ returnUrl } = {}) => {
      const body = { return_url: returnUrl ?? `${origin()}/billing` };
      return apiPost<PortalResponse, typeof body>(
        "/api/v1/billing/portal",
        body,
      );
    },
  });
}

/** Helper — open the portal session and redirect the browser. */
export async function redirectToPortal(
  mutateAsync: (input: PortalInput) => Promise<PortalResponse>,
  input: PortalInput = {},
): Promise<void> {
  const response = await mutateAsync(input);
  if (typeof window !== "undefined") {
    window.location.assign(response.url);
  }
}
