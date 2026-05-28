import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import BillingSubscribePage from "@/app/(app)/billing/subscribe/page";
import { metadata as subscribeMetadata } from "@/app/(app)/billing/subscribe/layout";

const replaceMock = vi.fn();
const pushMock = vi.fn();
const searchParamsGet = vi.fn<(key: string) => string | null>();

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push: pushMock, replace: replaceMock }),
  useSearchParams: () => ({ get: searchParamsGet }),
}));

const originalLocation = window.location;
const assignMock = vi.fn();

beforeEach(() => {
  replaceMock.mockReset();
  pushMock.mockReset();
  searchParamsGet.mockReset();
  searchParamsGet.mockReturnValue("pro");
  assignMock.mockReset();
  Object.defineProperty(window, "location", {
    configurable: true,
    value: { ...originalLocation, assign: assignMock, origin: "http://localhost:3000" },
  });
  process.env.NEXT_PUBLIC_STRIPE_PRICE_PRO = "price_pro_123";
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

describe("/billing/subscribe a11y + metadata", () => {
  it("layout exports metadata with title=Subscribe", () => {
    expect(subscribeMetadata.title).toBe("Subscribe");
  });

  it("renders exactly one <h1> with the page title", async () => {
    vi.spyOn(globalThis, "fetch").mockImplementation(
      async () =>
        new Response(JSON.stringify({ id: "cs", url: "https://stripe.test/cs" }), {
          status: 200,
          headers: { "content-type": "application/json" },
        }),
    );
    render(wrap(<BillingSubscribePage />));
    await waitFor(() => {
      const h1s = screen.getAllByRole("heading", { level: 1 });
      expect(h1s).toHaveLength(1);
      expect(h1s[0]).toHaveTextContent("Subscribe");
    });
  });

  it("error path still renders exactly one <h1>", async () => {
    delete process.env.NEXT_PUBLIC_STRIPE_PRICE_PRO;
    render(wrap(<BillingSubscribePage />));
    await waitFor(() => {
      const h1s = screen.getAllByRole("heading", { level: 1 });
      expect(h1s).toHaveLength(1);
      expect(h1s[0]).toHaveTextContent(/Couldn’t start checkout/);
    });
  });
});
