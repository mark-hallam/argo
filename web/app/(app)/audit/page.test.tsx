import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AuditFeedPage, AuditRow } from "@/lib/types/audit";

const replaceMock = vi.fn();
let searchParams = new URLSearchParams();

vi.mock("next/navigation", () => ({
  useRouter: () => ({
    replace: replaceMock,
    push: vi.fn(),
    prefetch: vi.fn(),
    refresh: vi.fn(),
    back: vi.fn(),
    forward: vi.fn(),
  }),
  usePathname: () => "/audit",
  useSearchParams: () => searchParams,
}));

type FeedShape = {
  data: { pages: AuditFeedPage[] } | undefined;
  isPending: boolean;
  isError: boolean;
  isFetching: boolean;
  isFetchingNextPage: boolean;
  hasNextPage: boolean;
  error: Error | null;
  fetchNextPage: () => void;
  refetch: () => void;
};
const useFeedMock = vi.fn<() => FeedShape>();
vi.mock("@/lib/hooks/use-audit-feed", () => ({
  useAuditFeed: () => useFeedMock(),
  auditFeedQueryKey: () => ["audit-feed"],
}));

vi.mock("@/lib/hooks/use-viewer-policy", async () => {
  const { VIEWER_POLICY_OPERATOR } = await import("@/lib/redaction/types");
  return { useViewerPolicy: () => VIEWER_POLICY_OPERATOR };
});

import AuditPage from "@/app/(app)/audit/page";

function makeRow(overrides: Partial<AuditRow> = {}): AuditRow {
  return {
    id: "act_xyz_001",
    timestamp: new Date(Date.now() - 7 * 60_000).toISOString(),
    worker: { id: "wkr_01J0PRIYA", name: "Priya" },
    actionType: "candidate_screened",
    verb: "screened Diego Alvarez against JD-2024-08",
    status: "succeeded",
    ...overrides,
  };
}

function feedState(overrides: Partial<FeedShape> = {}): FeedShape {
  return {
    data: { pages: [{ rows: [], nextCursor: null, totalPages: 1 }] },
    isPending: false,
    isError: false,
    isFetching: false,
    isFetchingNextPage: false,
    hasNextPage: false,
    error: null,
    fetchNextPage: vi.fn(),
    refetch: vi.fn(),
    ...overrides,
  };
}

beforeEach(() => {
  searchParams = new URLSearchParams();
  replaceMock.mockClear();
  useFeedMock.mockReset();
});

afterEach(() => {
  vi.clearAllMocks();
  vi.useRealTimers();
});

describe("AuditPage", () => {
  it("renders the page header (title, subtitle, action row, search)", () => {
    useFeedMock.mockReturnValue(feedState());
    render(<AuditPage />);
    expect(screen.getByRole("heading", { name: "Audit" })).toBeInTheDocument();
    expect(screen.getByTestId("audit-subtitle")).toHaveTextContent(
      /forensic \+ compliance view/i,
    );
    expect(screen.getByTestId("audit-filter-trigger")).toBeInTheDocument();
    expect(screen.getByTestId("audit-range-select")).toBeInTheDocument();
    expect(screen.getByTestId("audit-export-trigger")).toBeInTheDocument();
    expect(screen.getByTestId("audit-search-input")).toBeInTheDocument();
  });

  it("changing the time-range dropdown writes ?range= to the URL", async () => {
    const user = userEvent.setup();
    useFeedMock.mockReturnValue(feedState());
    render(<AuditPage />);
    await user.selectOptions(screen.getByTestId("audit-range-select"), "1h");
    expect(replaceMock).toHaveBeenCalled();
    const args = replaceMock.mock.calls.at(-1);
    expect(args?.[0]).toContain("range=1h");
  });

  it("selecting Custom range pops a placeholder toast and does NOT update URL", async () => {
    const user = userEvent.setup();
    useFeedMock.mockReturnValue(feedState());
    render(<AuditPage />);
    await user.selectOptions(screen.getByTestId("audit-range-select"), "custom");
    expect(replaceMock).not.toHaveBeenCalled();
    expect(screen.getByTestId("audit-toast")).toHaveTextContent(/E5-b/);
  });

  it("typing in search debounces a ?q= write to the URL", async () => {
    vi.useFakeTimers();
    useFeedMock.mockReturnValue(feedState());
    render(<AuditPage />);
    // fireEvent.change is synchronous; userEvent.type interleaves with the real
    // event loop and deadlocks under fake timers without setup gymnastics.
    fireEvent.change(screen.getByTestId("audit-search-input"), {
      target: { value: "Diego" },
    });
    expect(replaceMock).not.toHaveBeenCalled();
    await act(async () => {
      vi.advanceTimersByTime(220);
    });
    expect(replaceMock).toHaveBeenCalled();
    const args = replaceMock.mock.calls.at(-1);
    expect(args?.[0]).toMatch(/q=Diego/);
    vi.useRealTimers();
  });

  it("renders Load more — page n+1 of N and calls fetchNextPage on click", async () => {
    const user = userEvent.setup();
    const fetchNextPage = vi.fn();
    const rows = [makeRow({ id: "act_page1" })];
    useFeedMock.mockReturnValue(
      feedState({
        data: { pages: [{ rows, nextCursor: "50", totalPages: 3 }] },
        hasNextPage: true,
        fetchNextPage,
      }),
    );
    render(<AuditPage />);
    const btn = screen.getByTestId("audit-load-more");
    expect(btn).toHaveTextContent("Load more — page 2 of 3");
    await user.click(btn);
    expect(fetchNextPage).toHaveBeenCalledTimes(1);
  });

  it("clicking a row routes to /audit/action/<id>", () => {
    const rows = [makeRow({ id: "act_link_target" })];
    useFeedMock.mockReturnValue(
      feedState({ data: { pages: [{ rows, nextCursor: null, totalPages: 1 }] } }),
    );
    render(<AuditPage />);
    const link = screen.getByTestId("activity-row-act_link_target");
    expect(link).toHaveAttribute("href", "/audit/action/act_link_target");
  });

  it("renders the pristine empty state when no rows and no filters", () => {
    useFeedMock.mockReturnValue(
      feedState({ data: { pages: [{ rows: [], nextCursor: null, totalPages: 1 }] } }),
    );
    render(<AuditPage />);
    const empty = screen.getByTestId("audit-empty-pristine");
    expect(within(empty).getByText(/Nothing's happened yet\./)).toBeInTheDocument();
  });

  it("renders the filtered empty state when filters narrow to zero rows", () => {
    searchParams = new URLSearchParams("range=1h&status=refused");
    useFeedMock.mockReturnValue(
      feedState({ data: { pages: [{ rows: [], nextCursor: null, totalPages: 1 }] } }),
    );
    render(<AuditPage />);
    expect(screen.getByTestId("audit-empty-filtered")).toBeInTheDocument();
  });

  it("renders an error state with retry and calls refetch", async () => {
    const user = userEvent.setup();
    const refetch = vi.fn();
    useFeedMock.mockReturnValue(
      feedState({
        isError: true,
        data: undefined,
        error: new Error("boom"),
        refetch,
      }),
    );
    render(<AuditPage />);
    const err = screen.getByTestId("audit-error");
    expect(within(err).getByText(/Couldn't load audit/)).toBeInTheDocument();
    expect(within(err).getByText(/boom/)).toBeInTheDocument();
    await user.click(within(err).getByRole("button", { name: /Try again/ }));
    expect(refetch).toHaveBeenCalledTimes(1);
  });

  it("renders deep-link chips for ?worker / ?action_type / ?status and clears on click", async () => {
    const user = userEvent.setup();
    searchParams = new URLSearchParams("worker=wkr_01J0PRIYA&action_type=email_drafted&status=pending");
    useFeedMock.mockReturnValue(feedState());
    render(<AuditPage />);
    expect(screen.getByTestId("audit-chip-rail")).toBeInTheDocument();
    expect(screen.getByTestId("audit-chip-worker")).toHaveTextContent(/wkr_01J0PRIYA/);
    expect(screen.getByTestId("audit-chip-action_type")).toHaveTextContent(/email_drafted/);
    expect(screen.getByTestId("audit-chip-status")).toHaveTextContent(/pending/);
    await user.click(screen.getByTestId("audit-chip-worker"));
    expect(replaceMock).toHaveBeenCalled();
    const args = replaceMock.mock.calls.at(-1);
    expect(args?.[0]).not.toContain("worker=");
  });

  it("groups rows under TODAY / YESTERDAY headers in the feed", () => {
    const now = Date.now();
    const todayRow = makeRow({
      id: "act_today",
      timestamp: new Date(now - 30 * 60_000).toISOString(),
    });
    const yesterdayRow = makeRow({
      id: "act_yest",
      timestamp: new Date(now - 25 * 60 * 60_000).toISOString(),
    });
    useFeedMock.mockReturnValue(
      feedState({
        data: {
          pages: [{ rows: [todayRow, yesterdayRow], nextCursor: null, totalPages: 1 }],
        },
      }),
    );
    render(<AuditPage />);
    const feed = screen.getByTestId("audit-feed");
    expect(within(feed).getByText("TODAY")).toBeInTheDocument();
    expect(within(feed).getByText("YESTERDAY")).toBeInTheDocument();
    expect(within(feed).getByTestId("audit-row-wrapper-act_today")).toBeInTheDocument();
    expect(within(feed).getByTestId("audit-row-wrapper-act_yest")).toBeInTheDocument();
  });

  it("clicking the Export trigger opens the EvidenceBundleExporter dialog", async () => {
    const user = userEvent.setup();
    useFeedMock.mockReturnValue(feedState());
    const client = new QueryClient({
      defaultOptions: { queries: { retry: false, gcTime: 0 } },
    });
    render(
      <QueryClientProvider client={client}>
        <AuditPage />
      </QueryClientProvider>,
    );
    await user.click(screen.getByTestId("audit-export-trigger"));
    expect(await screen.findByTestId("export-dialog")).toBeInTheDocument();
  });
});
