import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// The Stripe price id is read at MODULE LOAD by `PRICE_BY_PLAN` in
// app/(app)/billing/subscribe/page.tsx (a static
// `process.env.NEXT_PUBLIC_STRIPE_PRICE_PRO` read). A `beforeEach` mutation
// lands too late — the module already captured `undefined`. `vi.hoisted` runs
// before the import below evaluates, so the Pro price resolves and the
// happy-path renders the single "Subscribe" <h1>. Enterprise is left UNSET so
// the error-path test (which deep-links to an unconfigured plan) still hits
// the "Couldn’t start checkout" shell.
vi.hoisted(() => {
  vi.stubEnv("NEXT_PUBLIC_STRIPE_PRICE_PRO", "price_pro_123");
  vi.stubEnv("NEXT_PUBLIC_STRIPE_PRICE_ENTERPRISE", "");
});

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
  // The Pro Stripe price id is stubbed at module load via vi.hoisted above
  // (PRICE_BY_PLAN reads it once at import time), so no per-test env set.
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
    // Deep-link to a plan whose Stripe price id is unconfigured. Enterprise
    // is left UNSET at module load (see vi.hoisted above), so PRICE_BY_PLAN
    // resolves undefined and the page renders the "Couldn’t start checkout"
    // error shell. (A runtime `delete` no longer works: PRICE_BY_PLAN reads
    // the env once at module-eval, the very behaviour the static-read fix
    // documents — so the test drives the genuinely-unconfigured plan instead.)
    searchParamsGet.mockReturnValue("enterprise");
    render(wrap(<BillingSubscribePage />));
    await waitFor(() => {
      const h1s = screen.getAllByRole("heading", { level: 1 });
      expect(h1s).toHaveLength(1);
      expect(h1s[0]).toHaveTextContent(/Couldn’t start checkout/);
    });
  });
});
