/**
 * Regression pin — `/workers` perpetual "Fetching the live worker roster."
 *
 * `/workers` (the nav-bar roster entrypoint that redirects to `/team` when
 * workers exist) once showed "Fetching the live worker roster." with NO
 * `/api/v1/workers` request and never redirected: the page's data fetch
 * (`useTeam`) was mounted INSIDE the `useSearchParams()` Suspense boundary,
 * which — sitting inside `RouteTransition`'s `<AnimatePresence>`
 * (`lib/motion/primitives.tsx`) — suspends on hydration and never commits,
 * so the React Query observer never subscribed, `queryFn` never fired, and
 * the `/team` redirect (also gated by the boundary) never ran. (Same
 * boundary bug as the `/team` Scene-1 hang fixed in `6a88284fb`.)
 *
 * The fix (web/app/(app)/workers/page.tsx): `useTeam()` is called in the
 * OUTER `WorkersPage` component, ABOVE the boundary. These tests pin that
 * contract so it cannot silently re-regress.
 */
import { render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Worker } from "@/lib/types/worker";

let suspendSearchParams = false;
let searchParams = new URLSearchParams();
const NEVER = new Promise<void>(() => {});
const replaceSpy = vi.fn();

vi.mock("next/navigation", () => ({
  useRouter: () => ({
    replace: replaceSpy,
    push: vi.fn(),
    prefetch: vi.fn(),
    refresh: vi.fn(),
    back: vi.fn(),
    forward: vi.fn(),
  }),
  usePathname: () => "/workers",
  useSearchParams: () => {
    if (suspendSearchParams) throw NEVER;
    return searchParams;
  },
}));

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

vi.mock("@/components/skycot/TeammateCard", () => ({
  TeammateCard: ({ worker }: { worker: Worker }) => (
    <div data-testid={`teammate-${worker.id}`}>{worker.name}</div>
  ),
}));
vi.mock("@/components/empty-state/HireStarterPrompts", () => ({
  HireStarterPrompts: () => <div data-testid="hire-starter-prompts" />,
}));

import WorkersPage from "./page";

const ROSTER: Worker[] = [
  { id: "wkr_grant", name: "Grant", role: { tagline: "GTM", vertical: "gtm" }, status: "working" },
];

beforeEach(() => {
  suspendSearchParams = false;
  searchParams = new URLSearchParams();
  replaceSpy.mockReset();
  useTeamSpy.mockReset();
  useTeamSpy.mockReturnValue({
    data: { workers: ROSTER, source: "gateway" },
    isPending: false,
    isError: false,
    refetch: vi.fn(),
  });
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("/workers fetch fires regardless of the useSearchParams Suspense boundary", () => {
  it("calls useTeam() on mount even when the inner useSearchParams boundary is SUSPENDED", () => {
    // The exact production failure condition. Before the fix, useTeam lived
    // inside the boundary and was never invoked → no /api/v1/workers request
    // → perpetual "Fetching the live worker roster." After the fix it is
    // hoisted above the boundary and MUST still be called.
    suspendSearchParams = true;
    render(<WorkersPage />);
    expect(useTeamSpy).toHaveBeenCalled();
  });

  it("redirects to /team once the roster resolves (no just-hired param)", async () => {
    render(<WorkersPage />);
    await waitFor(() => {
      expect(replaceSpy).toHaveBeenCalledWith("/team");
    });
  });

  it("renders the empty hire grid (not a hung spinner) when the query resolves with zero workers", () => {
    useTeamSpy.mockReturnValue({ data: { workers: [], source: "gateway" }, isPending: false });
    suspendSearchParams = true;
    render(<WorkersPage />);
    expect(screen.getByTestId("hire-starter-prompts")).toBeInTheDocument();
  });

  it("fires the fetch under the demo-live env triplet (auth-bypass + demo-mode + demo-live)", () => {
    vi.stubEnv("NODE_ENV", "development");
    vi.stubEnv("NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS", "1");
    vi.stubEnv("NEXT_PUBLIC_SKYCOT_DEMO_MODE", "1");
    vi.stubEnv("NEXT_PUBLIC_SKYCOT_DEMO_LIVE", "1");
    try {
      render(<WorkersPage />);
      expect(useTeamSpy).toHaveBeenCalled();
    } finally {
      vi.unstubAllEnvs();
    }
  });
});
