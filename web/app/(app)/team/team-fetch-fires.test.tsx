/**
 * Regression pin — Scene-1 `/team` perpetual-skeleton.
 *
 * The demo's opening shot (`/team`) once rendered loading skeletons
 * forever and the browser issued NO `/api/v1/workers` request: the page's
 * data fetch (`useTeam`) was mounted INSIDE the `useSearchParams()`
 * Suspense boundary, which — sitting inside `RouteTransition`'s
 * `<AnimatePresence>` — could suspend on hydration and never commit, so
 * the React Query observer never subscribed and `queryFn` never fired.
 *
 * The fix (web/app/(app)/team/page.tsx + web/lib/hooks/use-team.ts):
 * `useTeam()` is called in the OUTER `TeamPage` component, ABOVE the
 * Suspense boundary. These tests pin that contract so it cannot silently
 * re-regress:
 *
 *  1. `useTeam()` is invoked on mount even when the inner
 *     `useSearchParams()` boundary is SUSPENDED (the exact failure
 *     condition) — i.e. the fetch is not gated behind the boundary.
 *  2. The roster renders (not a perpetual skeleton) once the query
 *     resolves, including in the demo-live env triplet.
 */
import { render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Worker } from "@/lib/types/worker";

// --- Router / search-params mock ----------------------------------------
// `suspendSearchParams` lets a test put `useSearchParams()` into the
// never-resolving suspended state that wedged the page in production.
let suspendSearchParams = false;
let searchParams = new URLSearchParams();
// A promise that never resolves — mirrors the production hydration hang
// where the CSR-bailout Suspense boundary never commits.
const NEVER = new Promise<void>(() => {});

vi.mock("next/navigation", () => ({
  useRouter: () => ({
    replace: vi.fn(),
    push: vi.fn(),
    prefetch: vi.fn(),
    refresh: vi.fn(),
    back: vi.fn(),
    forward: vi.fn(),
  }),
  usePathname: () => "/team",
  useSearchParams: () => {
    if (suspendSearchParams) throw NEVER;
    return searchParams;
  },
}));

// --- useTeam SPY (not a stub of the data) --------------------------------
// We spy on the real hook contract: every test asserts WHETHER it was
// called. The return value is controlled per-test to drive render states.
type QueryShape = {
  data: { workers: Worker[]; source: "gateway" | "mock-offline" | "mock-bypass" } | undefined;
  isPending: boolean;
  isError?: boolean;
  refetch?: () => void;
};
const useTeamSpy = vi.fn<() => QueryShape>();
vi.mock("@/lib/hooks/use-team", () => ({
  useTeam: () => useTeamSpy(),
}));

// Stub the team-goals rollup hook so the card (above the Suspense
// boundary) doesn't fire a real fetch or need a QueryClientProvider.
vi.mock("@/lib/hooks/use-worker-scorecard", () => ({
  useTeamScorecard: () => ({ data: undefined, isLoading: false, isError: false }),
}));

const useAuditFeedSpy = vi.fn<() => { data: { pages: Array<{ rows: unknown[] }> } | undefined; isPending: boolean }>();
vi.mock("@/lib/hooks/use-audit-feed", () => ({
  useAuditFeed: () => useAuditFeedSpy(),
}));

vi.mock("@/components/skycot/TeammateCard", () => ({
  TeammateCard: ({ worker }: { worker: Worker }) => (
    <div data-testid={`teammate-${worker.id}`}>{worker.name}</div>
  ),
}));

import TeamPage from "./page";

const ROSTER: Worker[] = [
  { id: "wkr_grant", name: "Grant", role: { tagline: "GTM", vertical: "gtm" }, status: "working" },
  { id: "wkr_portia", name: "Portia", role: { tagline: "pitch", vertical: "pitch" }, status: "idle" },
];

beforeEach(() => {
  suspendSearchParams = false;
  searchParams = new URLSearchParams();
  useTeamSpy.mockReset();
  useTeamSpy.mockReturnValue({
    data: { workers: ROSTER, source: "gateway" },
    isPending: false,
    isError: false,
    refetch: vi.fn(),
  });
  useAuditFeedSpy.mockReset();
  useAuditFeedSpy.mockReturnValue({ data: { pages: [{ rows: [] }] }, isPending: false });
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("/team fetch fires regardless of the useSearchParams Suspense boundary", () => {
  it("calls useTeam() on mount even when the inner useSearchParams boundary is SUSPENDED", () => {
    // This is the exact production failure condition: the searchParams
    // boundary never resolves. Before the fix, useTeam lived inside that
    // boundary and was therefore never invoked → no /api/v1/workers
    // request → perpetual skeleton. After the fix it is hoisted above the
    // boundary and MUST still be called.
    suspendSearchParams = true;
    render(<TeamPage />);
    expect(useTeamSpy).toHaveBeenCalled();
    // The hero (also outside the boundary) renders from the resolved query
    // while the inner filter view is still suspended on the fallback.
    expect(
      screen.getByRole("heading", { level: 1, name: /teammates, one team\.|your team|your first teammate/i }),
    ).toBeInTheDocument();
  });

  it("renders the roster (not a perpetual skeleton) when the query has resolved", async () => {
    render(<TeamPage />);
    await waitFor(() => {
      expect(screen.getByTestId("teammate-wkr_grant")).toBeInTheDocument();
    });
    expect(screen.getByTestId("teammate-wkr_portia")).toBeInTheDocument();
    // No skeletons once data is present.
    expect(screen.queryByTestId("teammate-loading-0")).not.toBeInTheDocument();
  });

  it("shows the loading grid (never an empty/hire state) while the query is genuinely pending", () => {
    useTeamSpy.mockReturnValue({ data: undefined, isPending: true });
    render(<TeamPage />);
    expect(screen.getByTestId("teammate-loading-0")).toBeInTheDocument();
    // Pending must NOT collapse into the first-hire empty state — that
    // was the original camera-visible defect (#2444 / d81042118).
    expect(screen.queryByTestId("team-empty-state")).not.toBeInTheDocument();
  });

  it("fires the fetch under the demo-live env triplet (auth-bypass + demo-mode + demo-live)", () => {
    // The recording profile. `useTeam()` is mocked here, but the call
    // itself is the contract: the page must INVOKE the data hook in this
    // env so the real fetcher (which reads the live gateway under
    // demo-live — NO THEATER) gets its chance to run.
    vi.stubEnv("NODE_ENV", "development");
    vi.stubEnv("NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS", "1");
    vi.stubEnv("NEXT_PUBLIC_SKYCOT_DEMO_MODE", "1");
    vi.stubEnv("NEXT_PUBLIC_SKYCOT_DEMO_LIVE", "1");
    try {
      render(<TeamPage />);
      expect(useTeamSpy).toHaveBeenCalled();
      expect(useAuditFeedSpy).toHaveBeenCalled();
    } finally {
      vi.unstubAllEnvs();
    }
  });
});
