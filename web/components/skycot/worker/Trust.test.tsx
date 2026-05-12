import { render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { TrustGradeCard } from "@/components/skycot/worker/TrustGradeCard";
import { TrustGradeProgress } from "@/components/skycot/worker/TrustGradeProgress";
import {
  normalizeTrustGrade,
  type TrustGradeSnapshot,
} from "@/lib/api/worker-trust";
import { findSampleWorkerTrust } from "@/lib/sample-worker-trust";
import type { Worker } from "@/lib/types/worker";

vi.mock("next/navigation", () => ({
  useParams: () => ({ workerId: "wkr_01J0PRIYA" }),
  useRouter: () => ({
    replace: vi.fn(),
    push: vi.fn(),
    prefetch: vi.fn(),
    refresh: vi.fn(),
    back: vi.fn(),
    forward: vi.fn(),
  }),
  usePathname: () => "/workers/wkr_01J0PRIYA/trust",
  useSearchParams: () => new URLSearchParams(),
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

type TrustQueryShape = {
  data: { snapshot: TrustGradeSnapshot; source: "gateway" } | undefined;
  isPending: boolean;
  isError: boolean;
  error: Error | null;
  refetch: () => void;
};
const useTrustMock = vi.fn<() => TrustQueryShape>();
vi.mock("@/lib/hooks/use-worker-trust", () => ({
  useWorkerTrustGrade: () => useTrustMock(),
  workerTrustGradeQueryKey: () => ["worker-trust-grade"],
}));

import WorkerTrustPage from "@/app/(app)/workers/[workerId]/trust/page";

const PRIYA: Worker = {
  id: "wkr_01J0PRIYA",
  name: "Priya",
  role: { tagline: "screens candidates and books interviews", vertical: "recruiting" },
  status: "waiting",
};

function trustState(overrides: Partial<TrustQueryShape> = {}): TrustQueryShape {
  const snapshot = findSampleWorkerTrust("wkr_01J0PRIYA");
  return {
    data: { snapshot, source: "gateway" },
    isPending: false,
    isError: false,
    error: null,
    refetch: vi.fn(),
    ...overrides,
  };
}

beforeEach(() => {
  useWorkerMock.mockReset();
  useWorkerMock.mockReturnValue({ data: PRIYA, isPending: false, error: null });
  useTrustMock.mockReset();
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("WorkerTrustPage", () => {
  it("renders the trust-grade badge with the current label and tone", () => {
    useTrustMock.mockReturnValue(trustState());
    render(<WorkerTrustPage />);
    const badge = screen.getByTestId("trust-grade-badge");
    expect(badge).toHaveTextContent("Tenant");
    expect(badge).toHaveAttribute("data-grade", "tenant");
  });

  it("renders the progress bar with criteria-met markers", () => {
    useTrustMock.mockReturnValue(trustState());
    render(<WorkerTrustPage />);
    const list = screen.getByTestId("trust-grade-criteria-list");
    const items = within(list).getAllByTestId("trust-grade-criterion");
    expect(items.length).toBeGreaterThanOrEqual(2);
    // Priya's seed has at least one met + at least one unmet.
    expect(items.some((el) => el.getAttribute("data-met") === "true")).toBe(true);
    expect(items.some((el) => el.getAttribute("data-met") === null)).toBe(true);
  });

  it("shows the at-max banner and hides the criteria list when at the top rung", () => {
    const max: TrustGradeSnapshot = {
      ...findSampleWorkerTrust("wkr_01J0NORI"),
    };
    useTrustMock.mockReturnValue(trustState({ data: { snapshot: max, source: "gateway" } }));
    render(<WorkerTrustPage />);
    expect(screen.getByTestId("trust-grade-at-max")).toBeInTheDocument();
    expect(screen.queryByTestId("trust-grade-criteria-list")).not.toBeInTheDocument();
  });

  it("renders no edit affordance anywhere on the trust tab", () => {
    useTrustMock.mockReturnValue(trustState());
    render(<WorkerTrustPage />);
    // Read-only contract: the panel surface contains no form
    // controls except the error-state "Try again" — none on the
    // happy path. Inputs / selects / textareas should be absent.
    const panel = screen.getByTestId("worker-tab-panel-trust");
    expect(within(panel).queryAllByRole("button")).toEqual([]);
    expect(within(panel).queryAllByRole("textbox")).toEqual([]);
    expect(within(panel).queryAllByRole("combobox")).toEqual([]);
  });

  it("shows the loading state while the trust query is pending", () => {
    useTrustMock.mockReturnValue(trustState({ data: undefined, isPending: true }));
    render(<WorkerTrustPage />);
    expect(screen.getByTestId("trust-grade-loading")).toBeInTheDocument();
  });

  it("shows the error state and a retry button when the query fails", () => {
    const refetch = vi.fn();
    useTrustMock.mockReturnValue(
      trustState({
        isPending: false,
        isError: true,
        data: undefined,
        error: new Error("network down"),
        refetch,
      }),
    );
    render(<WorkerTrustPage />);
    const err = screen.getByTestId("trust-grade-error");
    expect(within(err).getByText(/Couldn't load trust grade/)).toBeInTheDocument();
    expect(within(err).getByText(/network down/)).toBeInTheDocument();
  });
});

describe("normalizeTrustGrade", () => {
  it("maps the gateway snake_case body to the camelCase snapshot", () => {
    const body = {
      worker_id: "wkr_xyz",
      current_grade: "tenant",
      current_grade_label: "Tenant",
      current_grade_tone: "warning",
      current_grade_index: 2,
      next_grade: "published_verified",
      next_grade_label: "Published verified",
      at_max_grade: false,
      signals: {
        trust_grade_value: 2,
        has_been_used: true,
        last_active_days_ago: 5,
      },
      next_grade_criteria: [
        { name: "criterion-a", current_value: "yes", target_value: "yes", met: true },
        { name: "criterion-b", current_value: "no", target_value: "yes", met: false },
      ],
    };
    const out = normalizeTrustGrade(body, "wkr_fallback");
    expect(out.workerId).toBe("wkr_xyz");
    expect(out.currentGrade).toBe("tenant");
    expect(out.currentGradeTone).toBe("warning");
    expect(out.atMaxGrade).toBe(false);
    expect(out.signals.trustGradeValue).toBe(2);
    expect(out.signals.lastActiveDaysAgo).toBe(5);
    expect(out.nextGradeCriteria).toHaveLength(2);
    expect(out.nextGradeCriteria[0]?.met).toBe(true);
    expect(out.nextGradeCriteria[1]?.met).toBe(false);
  });

  it("falls back to safe defaults when fields are missing or malformed", () => {
    const out = normalizeTrustGrade({}, "wkr_fallback");
    expect(out.workerId).toBe("wkr_fallback");
    expect(out.currentGrade).toBe("learned_unreviewed");
    expect(out.currentGradeTone).toBe("warning");
    expect(out.nextGrade).toBe(null);
    expect(out.atMaxGrade).toBe(false);
    expect(out.signals.trustGradeValue).toBe(0);
    expect(out.signals.hasBeenUsed).toBe(false);
    expect(out.signals.lastActiveDaysAgo).toBe(null);
    expect(out.nextGradeCriteria).toEqual([]);
  });
});

describe("TrustGradeCard / TrustGradeProgress (unit)", () => {
  it("renders the active rung in the 5-step ladder", () => {
    const snap = findSampleWorkerTrust("wkr_01J0MARIAH");
    render(<TrustGradeCard snapshot={snap} workerName="Mariah" />);
    const rung = screen.getByTestId(`trust-grade-rung-${snap.currentGrade}`);
    expect(rung).toHaveAttribute("data-active", "true");
  });

  it("the progress component reports a met-count summary derived from criteria", () => {
    const snap = findSampleWorkerTrust("wkr_01J0PRIYA");
    render(
      <TrustGradeProgress
        currentGrade={snap.currentGrade}
        currentGradeIndex={snap.currentGradeIndex}
        nextGrade={snap.nextGrade}
        nextGradeLabel={snap.nextGradeLabel}
        atMaxGrade={snap.atMaxGrade}
        criteria={snap.nextGradeCriteria}
      />,
    );
    const summary = screen.getByTestId("trust-grade-met-summary");
    const met = snap.nextGradeCriteria.filter((c) => c.met).length;
    expect(summary).toHaveTextContent(`${met} of ${snap.nextGradeCriteria.length} met`);
  });
});
