/**
 * Regression pin — Scene-6 `/audit` perpetual-skeleton.
 *
 * `/audit` (the demo's audit chain-explorer beat) once rendered
 * "Loading audit timeline…" forever and the browser issued NO
 * `/api/v1/audit` request: the page's feed fetch (`useAuditFeed`) was
 * mounted INSIDE the `useSearchParams()` Suspense boundary, which — sitting
 * inside `RouteTransition`'s `<AnimatePresence>` (`lib/motion/primitives.tsx`)
 * — suspends on client hydration and never commits, so the React Query
 * observer never subscribed and `queryFn` never fired. (Verified in the live
 * demo-live SPA: 14s on the page, zero audit requests — sibling of the
 * `/team` Scene-1 hang fixed in `6a88284fb`.)
 *
 * The fix (web/app/(app)/audit/page.tsx + web/app/(app)/audit/use-audit-url-state.ts):
 * `useAuditFeed()` + the freshness probe are called in the OUTER `AuditPage`
 * component, ABOVE the Suspense boundary, reading the feed's `range`/`q` via
 * the non-suspending `useAuditFeedFilters` (a `useSyncExternalStore` over
 * `window.location.search`) instead of `useSearchParams`. These tests pin
 * that contract so it cannot silently re-regress:
 *
 *  1. `useAuditFeed()` is invoked on mount even when the inner
 *     `useSearchParams()` boundary is SUSPENDED (the exact failure
 *     condition) — i.e. the feed fetch is not gated behind the boundary.
 *  2. The timeline rows render (not a perpetual "Loading…") once the query
 *     resolves, including with the inner boundary still suspended.
 *  3. The fetch fires under the demo-live env triplet (NO THEATER — the real
 *     fetcher reads the live gateway under demo-live).
 */
import { render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AuditRow } from "@/lib/types/audit";

// --- Router / search-params mock ----------------------------------------
// `suspendSearchParams` puts `useSearchParams()` into the never-resolving
// suspended state that wedged the page in production.
let suspendSearchParams = false;
let searchParams = new URLSearchParams();
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
  usePathname: () => "/audit",
  useSearchParams: () => {
    if (suspendSearchParams) throw NEVER;
    return searchParams;
  },
}));

// --- useAuditFeed SPY (not a stub of the data) ---------------------------
// We spy on the real hook contract: every test asserts WHETHER it was
// called. The return value is controlled per-test to drive render states.
type FeedShape = {
  data: { pages: Array<{ rows: AuditRow[]; nextCursor: string | null; totalPages?: number }> } | undefined;
  isPending: boolean;
  isError?: boolean;
  error?: Error;
  hasNextPage?: boolean;
  isFetchingNextPage?: boolean;
  fetchNextPage?: () => void;
  refetch?: () => void;
};
const useAuditFeedSpy = vi.fn<() => FeedShape>();
const useWorkspaceFreshnessSpy = vi.fn<() => "loading" | "fresh" | "active">();
vi.mock("@/lib/hooks/use-audit-feed", () => ({
  useAuditFeed: () => useAuditFeedSpy(),
  useAuditWorkspaceFreshness: () => useWorkspaceFreshnessSpy(),
}));

// Heavy presentational children are stubbed to keep the test focused on the
// fetch-fires contract (not on the chain-explorer hero / popover internals).
vi.mock("@/components/skycot/audit/AuditChainExplorerHero", () => ({
  AuditChainExplorerHero: () => <div data-testid="audit-hero" />,
}));
vi.mock("@/components/skycot/audit/AuditPageHeader", () => ({
  AuditPageHeader: () => <div data-testid="audit-header" />,
}));
vi.mock("@/components/skycot/audit/AuditDayGroup", () => ({
  AuditDayGroup: ({ bucket }: { bucket: { key: string; rows: AuditRow[] } }) => (
    <div data-testid={`audit-day-${bucket.key}`}>
      {bucket.rows.map((r) => (
        <div key={r.id} data-testid={`audit-row-${r.id}`}>
          {r.id}
        </div>
      ))}
    </div>
  ),
}));
vi.mock("@/components/skycot/audit/AuditFilterChipRail", () => ({
  AuditFilterChipRail: () => null,
}));
vi.mock("@/components/skycot/audit/AuditFilterPopover", () => ({
  AuditFilterPopover: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));
vi.mock("@/components/skycot/audit/EvidenceBundleExporter", () => ({
  EvidenceBundleExporter: () => null,
}));
vi.mock("@/components/skycot/audit/RedactionBanner", () => ({
  RedactionBanner: () => null,
}));
vi.mock("@/lib/hooks/use-viewer-policy", () => ({
  useViewerPolicy: () => ({ permissions: [], tenantId: "tenant_test", tenantLabel: "Test" }),
}));

import AuditPage from "./page";

function row(id: string): AuditRow {
  return {
    id,
    timestamp: new Date().toISOString(),
    worker: { id: "wkr_1", name: "Mariah" },
    actionType: "message",
    verb: `action ${id}`,
    status: "succeeded",
  };
}

const PAGE_WITH_ROWS = {
  pages: [{ rows: [row("a1"), row("a2")], nextCursor: null, totalPages: 1 }],
};

beforeEach(() => {
  suspendSearchParams = false;
  searchParams = new URLSearchParams();
  useAuditFeedSpy.mockReset();
  useAuditFeedSpy.mockReturnValue({
    data: PAGE_WITH_ROWS,
    isPending: false,
    isError: false,
    hasNextPage: false,
    isFetchingNextPage: false,
    fetchNextPage: vi.fn(),
    refetch: vi.fn(),
  });
  useWorkspaceFreshnessSpy.mockReset();
  useWorkspaceFreshnessSpy.mockReturnValue("active");
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("/audit feed fetch fires regardless of the useSearchParams Suspense boundary", () => {
  it("calls useAuditFeed() on mount even when the inner useSearchParams boundary is SUSPENDED", () => {
    // The exact production failure condition: the searchParams boundary
    // never resolves. Before the fix, useAuditFeed lived inside that boundary
    // and was therefore never invoked → no /api/v1/audit request → perpetual
    // "Loading audit timeline…". After the fix it is hoisted above the
    // boundary and MUST still be called.
    suspendSearchParams = true;
    render(<AuditPage />);
    expect(useAuditFeedSpy).toHaveBeenCalled();
  });

  it("renders the timeline rows from the already-mounted query while the boundary is suspended", () => {
    // The fallback renders the SAME timeline body driven by the hoisted feed,
    // so rows are never hidden behind the boundary resolving.
    suspendSearchParams = true;
    render(<AuditPage />);
    expect(screen.getByTestId("audit-row-a1")).toBeInTheDocument();
    expect(screen.getByTestId("audit-row-a2")).toBeInTheDocument();
    expect(screen.queryByText(/Loading audit timeline/i)).not.toBeInTheDocument();
  });

  it("renders the timeline rows once the boundary commits (query resolved)", async () => {
    render(<AuditPage />);
    await waitFor(() => {
      expect(screen.getByTestId("audit-row-a1")).toBeInTheDocument();
    });
    expect(screen.getByTestId("audit-row-a2")).toBeInTheDocument();
  });

  it("shows the loading state (never a hung blank) while the feed is genuinely pending", () => {
    useAuditFeedSpy.mockReturnValue({ data: undefined, isPending: true });
    suspendSearchParams = true;
    render(<AuditPage />);
    expect(screen.getByText(/Loading audit timeline/i)).toBeInTheDocument();
  });

  it("fires the fetch under the demo-live env triplet (auth-bypass + demo-mode + demo-live)", () => {
    // The recording profile. The CALL itself is the contract: the page must
    // INVOKE the feed hook in this env so the real fetcher (which reads the
    // live gateway under demo-live — NO THEATER) gets its chance to run.
    vi.stubEnv("NODE_ENV", "development");
    vi.stubEnv("NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS", "1");
    vi.stubEnv("NEXT_PUBLIC_SKYCOT_DEMO_MODE", "1");
    vi.stubEnv("NEXT_PUBLIC_SKYCOT_DEMO_LIVE", "1");
    try {
      render(<AuditPage />);
      expect(useAuditFeedSpy).toHaveBeenCalled();
    } finally {
      vi.unstubAllEnvs();
    }
  });
});
