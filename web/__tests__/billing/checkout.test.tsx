import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import BillingPage from "@/app/(app)/billing/page";

const assignMock = vi.fn();
const originalLocation = window.location;

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push: vi.fn(), replace: vi.fn() }),
}));

beforeEach(() => {
  assignMock.mockReset();
  Object.defineProperty(window, "location", {
    configurable: true,
    value: {
      ...originalLocation,
      assign: assignMock,
      origin: "http://localhost:3000",
    },
  });
  process.env.NEXT_PUBLIC_STRIPE_PRICE_PRO = "price_pro_123";
  delete process.env.NEXT_PUBLIC_STRIPE_PRICE_ENTERPRISE;
});

afterEach(() => {
  vi.restoreAllMocks();
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

function mockFetch(handlers: Record<string, (init?: RequestInit) => Promise<Response>>) {
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

describe("BillingPage", () => {
  it("renders the active tier and usage summary", async () => {
    mockFetch({
      "/api/v1/tenant/usage": async () =>
        jsonResponse({
          tier: "free",
          display_name: "Free",
          used: 5,
          limit: 100,
          remaining: 95,
          resets_at: "2026-06-01T00:00:00Z",
        }),
    });
    render(wrap(<BillingPage />));
    await waitFor(() =>
      expect(screen.getByText(/Used 5 of 100 actions/)).toBeInTheDocument(),
    );
  });

  it("redirects to Stripe Checkout when an upgrade plan is clicked", async () => {
    mockFetch({
      "/api/v1/tenant/usage": async () =>
        jsonResponse({
          tier: "free",
          display_name: "Free",
          used: 0,
          limit: 100,
          remaining: 100,
          resets_at: "2026-06-01T00:00:00Z",
        }),
      "/api/v1/billing/checkout": async () =>
        jsonResponse({ id: "cs_test_x", url: "https://stripe.test/cs_test_x" }),
    });
    render(wrap(<BillingPage />));
    await screen.findByText(/Used 0 of 100/);
    const upgrade = await screen.findByRole("button", { name: /Upgrade to Pro/ });
    await userEvent.click(upgrade);
    await waitFor(() =>
      expect(assignMock).toHaveBeenCalledWith("https://stripe.test/cs_test_x"),
    );
  });

  it("disables the upgrade button when no Stripe price id is configured", async () => {
    mockFetch({
      "/api/v1/tenant/usage": async () =>
        jsonResponse({
          tier: "pro",
          display_name: "Pro",
          used: 12,
          limit: 5000,
          remaining: 4988,
          resets_at: "2026-06-01T00:00:00Z",
        }),
    });
    render(wrap(<BillingPage />));
    await screen.findByText(/Used 12 of 5,000/);
    const ent = screen.getByRole("button", { name: /Contact sales/ });
    expect(ent).toBeDisabled();
  });

  it("opens the portal when Manage subscription is clicked", async () => {
    mockFetch({
      "/api/v1/tenant/usage": async () =>
        jsonResponse({
          tier: "pro",
          display_name: "Pro",
          used: 0,
          limit: 5000,
          remaining: 5000,
          resets_at: "2026-06-01T00:00:00Z",
        }),
      "/api/v1/billing/portal": async () =>
        jsonResponse({ id: "bps_test", url: "https://stripe.test/bps_test" }),
    });
    render(wrap(<BillingPage />));
    await screen.findByText(/Used 0 of 5,000/);
    const manage = await screen.findByRole("button", { name: /Manage subscription/ });
    await userEvent.click(manage);
    await waitFor(() =>
      expect(assignMock).toHaveBeenCalledWith("https://stripe.test/bps_test"),
    );
  });
});
