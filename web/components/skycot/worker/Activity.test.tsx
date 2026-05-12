import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type {
  WorkerActivity,
  WorkerActivityPage as WorkerActivityPageType,
} from "@/lib/api/worker-activity";
import type { Worker } from "@/lib/types/worker";

const replaceMock = vi.fn();
let searchParams = new URLSearchParams();

vi.mock("next/navigation", () => ({
  useParams: () => ({ workerId: "wkr_01J0PRIYA" }),
  useRouter: () => ({
    replace: replaceMock,
    push: vi.fn(),
    prefetch: vi.fn(),
    refresh: vi.fn(),
    back: vi.fn(),
    forward: vi.fn(),
  }),
  usePathname: () => "/workers/wkr_01J0PRIYA/activity",
  useSearchParams: () => searchParams,
}));

type WorkerQueryShape = {
  data: Worker | undefined;
  isPending: boolean;
  error?: Error | null;
};
const useWorkerMock = vi.fn<() => WorkerQueryShape>();
vi.mock("@/lib/hooks/use-worker", () => ({
  useWorker: () => useWorkerMock(),
}));

type ActivityQueryShape = {
  data: { pages: WorkerActivityPageType[] } | undefined;
  isPending: boolean;
  isError: boolean;
  isFetching: boolean;
  isFetchingNextPage: boolean;
  hasNextPage: boolean;
  error: Error | null;
  fetchNextPage: () => void;
  refetch: () => void;
};
const useActivityMock = vi.fn<() => ActivityQueryShape>();
vi.mock("@/lib/hooks/use-worker-activity", () => ({
  useWorkerActivity: () => useActivityMock(),
  workerActivityQueryKey: () => ["worker-activity"],
}));

import WorkerActivityPage from "@/app/(app)/workers/[workerId]/activity/page";

const WORKER: Worker = {
  id: "wkr_01J0PRIYA",
  name: "Priya",
  role: { tagline: "screens candidates and books interviews", vertical: "recruiting" },
  status: "waiting",
};

function makeRow(overrides: Partial<WorkerActivity> = {}): WorkerActivity {
  return {
    id: "act_001",
    workerId: "wkr_01J0PRIYA",
    workerName: "Priya",
    workerInitials: "PR",
    timestamp: new Date(Date.now() - 5 * 60_000).toISOString(),
    verb: "screened candidate #4821",
    actionType: "message_sent",
    status: "succeeded",
    ...overrides,
  };
}

function activityState(overrides: Partial<ActivityQueryShape> = {}): ActivityQueryShape {
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
  useWorkerMock.mockReset();
  useWorkerMock.mockReturnValue({ data: WORKER, isPending: false, error: null });
  useActivityMock.mockReset();
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("WorkerActivityPage", () => {
  it("renders the feed with one row per audit entry", async () => {
    const rows = [
      makeRow({ id: "act_a", verb: "screened candidate #4821" }),
      makeRow({ id: "act_b", verb: "booked interview with Diego Alvarez" }),
    ];
    useActivityMock.mockReturnValue(
      activityState({ data: { pages: [{ rows, nextCursor: null, totalPages: 1 }] } }),
    );
    render(<WorkerActivityPage />);
    const feed = screen.getByTestId("activity-feed");
    expect(within(feed).getByTestId("activity-row-act_a")).toBeInTheDocument();
    expect(within(feed).getByTestId("activity-row-act_b")).toBeInTheDocument();
    expect(within(feed).getByText(/screened candidate/)).toBeInTheDocument();
    expect(within(feed).getByText(/Diego Alvarez/)).toBeInTheDocument();
  });

  it("clicking a row routes to /audit/action/<id>", async () => {
    const rows = [makeRow({ id: "act_xyz" })];
    useActivityMock.mockReturnValue(
      activityState({ data: { pages: [{ rows, nextCursor: null, totalPages: 1 }] } }),
    );
    render(<WorkerActivityPage />);
    const row = screen.getByTestId("activity-row-act_xyz");
    expect(row).toHaveAttribute("href", "/audit/action/act_xyz");
  });

  it("changing a filter updates the URL via router.replace", async () => {
    const user = userEvent.setup();
    useActivityMock.mockReturnValue(activityState());
    render(<WorkerActivityPage />);
    await user.selectOptions(screen.getByTestId("activity-filter-range"), "1h");
    expect(replaceMock).toHaveBeenCalled();
    const args = replaceMock.mock.calls.at(-1);
    expect(args?.[0]).toContain("range=1h");
  });

  it("changing the action-type filter sets ?type=… in the URL", async () => {
    const user = userEvent.setup();
    useActivityMock.mockReturnValue(activityState());
    render(<WorkerActivityPage />);
    await user.selectOptions(
      screen.getByTestId("activity-filter-type"),
      "approval_asked",
    );
    expect(replaceMock).toHaveBeenCalled();
    const args = replaceMock.mock.calls.at(-1);
    expect(args?.[0]).toContain("type=approval_asked");
  });

  it("renders the pristine empty state when no rows and filters at default", async () => {
    useActivityMock.mockReturnValue(
      activityState({ data: { pages: [{ rows: [], nextCursor: null, totalPages: 1 }] } }),
    );
    render(<WorkerActivityPage />);
    const empty = screen.getByTestId("activity-empty-pristine");
    expect(within(empty).getByText(/No activity yet\./)).toBeInTheDocument();
    expect(within(empty).getByText(/Priya/)).toBeInTheDocument();
  });

  it("renders the filtered empty state with a Clear filters button when filters are active", async () => {
    const user = userEvent.setup();
    searchParams = new URLSearchParams("range=1h&type=escalation");
    useActivityMock.mockReturnValue(
      activityState({ data: { pages: [{ rows: [], nextCursor: null, totalPages: 1 }] } }),
    );
    render(<WorkerActivityPage />);
    const empty = screen.getByTestId("activity-empty-filtered");
    expect(within(empty).getByText(/No matching activity/)).toBeInTheDocument();
    await user.click(screen.getByTestId("activity-clear-filters"));
    expect(replaceMock).toHaveBeenCalledWith("?", { scroll: false });
  });

  it("renders a loading state while the query is pending", async () => {
    useActivityMock.mockReturnValue(
      activityState({ isPending: true, data: undefined }),
    );
    render(<WorkerActivityPage />);
    expect(screen.getByTestId("activity-loading")).toBeInTheDocument();
  });

  it("renders an error state and retry button when the query fails", async () => {
    const user = userEvent.setup();
    const refetch = vi.fn();
    useActivityMock.mockReturnValue(
      activityState({
        isError: true,
        data: undefined,
        error: new Error("boom"),
        refetch,
      }),
    );
    render(<WorkerActivityPage />);
    const err = screen.getByTestId("activity-error");
    expect(within(err).getByText(/Couldn't load activity/)).toBeInTheDocument();
    expect(within(err).getByText(/boom/)).toBeInTheDocument();
    await user.click(within(err).getByRole("button", { name: /Try again/ }));
    expect(refetch).toHaveBeenCalledTimes(1);
  });

  it("renders Load more and calls fetchNextPage when clicked", async () => {
    const user = userEvent.setup();
    const fetchNextPage = vi.fn();
    const rows = [makeRow({ id: "act_pageone" })];
    useActivityMock.mockReturnValue(
      activityState({
        data: { pages: [{ rows, nextCursor: "50", totalPages: 3 }] },
        hasNextPage: true,
        fetchNextPage,
      }),
    );
    render(<WorkerActivityPage />);
    const loadMore = screen.getByTestId("activity-load-more");
    expect(loadMore).toHaveTextContent("Load more — page 2 of 3");
    await user.click(loadMore);
    expect(fetchNextPage).toHaveBeenCalledTimes(1);
  });

  it("Open-in-Audit link preserves the current time range filter", async () => {
    searchParams = new URLSearchParams("range=30d");
    useActivityMock.mockReturnValue(activityState());
    render(<WorkerActivityPage />);
    const link = screen.getByTestId("activity-open-audit");
    expect(link).toHaveAttribute(
      "href",
      "/audit?worker=wkr_01J0PRIYA&range=30d",
    );
  });
});
