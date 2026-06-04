import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// Signed-in session — the Pro CTA renders as a checkout button only when
// authenticated; signed-out it stays a Link to signup.
vi.mock("@/lib/skycot/use-session", () => ({
  useSkycotSession: () => ({
    status: "authenticated",
    isDemoMode: false,
    email: "owner@acme.test",
    displayName: "Owner",
    tenantId: "tenant_acme",
    workspaceLabel: "Acme · Owner",
    role: "owner",
  }),
}));

vi.mock("next/navigation", () => ({
  useSearchParams: () => ({ get: () => null }),
}));

const setSelectedPlanMock = vi.fn();
vi.mock("@/lib/onboarding/use-onboarding-draft", () => ({
  useOnboardingDraft: (sel: (s: unknown) => unknown) =>
    sel({ setSelectedPlan: setSelectedPlanMock }),
}));

// Marketing chrome pulls in unrelated import chains; stub it out.
vi.mock("@/components/marketing/MarketingNav", () => ({
  MarketingNav: () => null,
}));
vi.mock("@/components/marketing/MarketingFooter", () => ({
  MarketingFooter: () => null,
}));

import PricingPage from "@/app/pricing/page";

const assignMock = vi.fn();
const originalLocation = window.location;

beforeEach(() => {
  assignMock.mockReset();
  setSelectedPlanMock.mockReset();
  Object.defineProperty(window, "location", {
    configurable: true,
    value: {
      ...originalLocation,
      assign: assignMock,
      origin: "http://localhost:3000",
    },
  });
  process.env.NEXT_PUBLIC_STRIPE_PRICE_PRO = "price_pro_123";
});

afterEach(() => {
  vi.restoreAllMocks();
  delete process.env.NEXT_PUBLIC_STRIPE_PRICE_PRO;
  Object.defineProperty(window, "location", {
    configurable: true,
    value: originalLocation,
  });
});

function wrap(ui: ReactNode) {
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0 },
      mutations: { retry: false },
    },
  });
  return <QueryClientProvider client={client}>{ui}</QueryClientProvider>;
}

function mockFetch(
  handlers: Record<string, (init?: RequestInit) => Promise<Response>>,
) {
  vi.spyOn(globalThis, "fetch").mockImplementation(async (input, init) => {
    const url = typeof input === "string" ? input : input.toString();
    for (const [pattern, handler] of Object.entries(handlers)) {
      if (url.includes(pattern)) return handler(init);
    }
    return new Response("not found", { status: 404 });
  });
}

function jsonResponse(body: unknown, init?: ResponseInit): Response {
  return new Response(JSON.stringify(body), {
    status: 200,
    headers: { "content-type": "application/json" },
    ...init,
  });
}

describe("PricingPage — Pro CTA → Stripe Checkout", () => {
  it("invokes the checkout endpoint and redirects to the hosted URL", async () => {
    mockFetch({
      "/api/v1/billing/checkout": async () =>
        jsonResponse({ id: "cs_test_x", url: "https://stripe.test/cs_test_x" }),
    });
    render(wrap(<PricingPage />));
    const pro = await screen.findByTestId("pricing-cta-pro");
    await userEvent.click(pro);
    await waitFor(() =>
      expect(assignMock).toHaveBeenCalledWith("https://stripe.test/cs_test_x"),
    );
  });

  it("shows a truthful error (never a fake success) when Stripe isn't configured", async () => {
    delete process.env.NEXT_PUBLIC_STRIPE_PRICE_PRO;
    mockFetch({});
    render(wrap(<PricingPage />));
    const pro = await screen.findByTestId("pricing-cta-pro");
    await userEvent.click(pro);
    const err = await screen.findByTestId("pricing-checkout-error");
    expect(err.textContent).toMatch(/still being set up/i);
    expect(assignMock).not.toHaveBeenCalled();
  });

  it("surfaces a truthful error when the checkout call fails", async () => {
    mockFetch({
      "/api/v1/billing/checkout": async () =>
        jsonResponse({ error: "stripe unavailable" }, { status: 502 }),
    });
    render(wrap(<PricingPage />));
    const pro = await screen.findByTestId("pricing-cta-pro");
    await userEvent.click(pro);
    await screen.findByTestId("pricing-checkout-error");
    expect(assignMock).not.toHaveBeenCalled();
  });

  it("keeps Free routing to onboarding (no charge) and Enterprise as the mailto path", async () => {
    mockFetch({});
    render(wrap(<PricingPage />));
    // Signed-in Free routes to onboarding (signed-out it would be
    // /auth/signup?plan=free); either way it never hits checkout.
    const free = await screen.findByTestId("pricing-cta-free");
    expect(free.getAttribute("href")).toBe("/onboarding/welcome");
    expect(free.tagName).toBe("A");
    const ent = await screen.findByTestId("pricing-cta-enterprise");
    expect(ent.getAttribute("href")).toContain("mailto:hello@skycot.com");
    expect(assignMock).not.toHaveBeenCalled();
  });
});
