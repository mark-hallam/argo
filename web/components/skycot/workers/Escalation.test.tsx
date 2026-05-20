import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { EscalationMatrix } from "@/components/skycot/workers/EscalationMatrix";
import { TrustOverTimeChart } from "@/components/skycot/workers/TrustOverTimeChart";
import {
  REFERENCE_NOW,
  findSampleTrustHistory,
  gradeForValue,
  SAMPLE_WORKER_TRUST_HISTORY,
} from "@/lib/sample-worker-trust-history";
import type { Worker } from "@/lib/types/worker";

vi.mock("next/navigation", () => ({
  useParams: () => ({ workerId: "wkr_01J0MARIAH" }),
  useRouter: () => ({
    replace: vi.fn(),
    push: vi.fn(),
    prefetch: vi.fn(),
    refresh: vi.fn(),
    back: vi.fn(),
    forward: vi.fn(),
  }),
  usePathname: () => "/workers/wkr_01J0MARIAH/escalation",
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

import WorkerEscalationPage from "@/app/(app)/workers/[workerId]/escalation/page";

const MARIAH: Worker = {
  id: "wkr_01J0MARIAH",
  name: "Mariah",
  role: { tagline: "lists items and screens buyers", vertical: "marketplace" },
  status: "working",
};

beforeEach(() => {
  useWorkerMock.mockReset();
  useWorkerMock.mockReturnValue({ data: MARIAH, isPending: false, error: null });
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("WorkerEscalationPage (Mariah)", () => {
  it("returns a 200-equivalent panel with at least 3 escalation rules", () => {
    render(<WorkerEscalationPage />);
    const panel = screen.getByTestId("worker-tab-panel-escalation");
    expect(panel).toBeInTheDocument();
    const rules = within(panel).getAllByTestId("escalation-rule");
    expect(rules.length).toBeGreaterThanOrEqual(3);
  });

  it("renders the when-to-escalate matrix with at least 3 routed rows", () => {
    render(<WorkerEscalationPage />);
    const matrix = screen.getByTestId("escalation-matrix");
    const rows = within(matrix).getAllByTestId("escalation-matrix-row");
    expect(rows.length).toBeGreaterThanOrEqual(3);
    rows.forEach((row) => {
      expect(within(row).getByTestId("escalation-matrix-routes-to")).toBeInTheDocument();
    });
  });

  it("renders escalation history entries with audit-row links", () => {
    render(<WorkerEscalationPage />);
    const historySection = screen.getByTestId("escalation-history");
    const links = within(historySection).getAllByTestId(
      "escalation-history-audit-link",
    );
    expect(links.length).toBeGreaterThanOrEqual(1);
    links.forEach((link) => {
      expect(link.getAttribute("href")).toMatch(/^\/audit\/action\//);
    });
  });

  it("renders the trust-over-time chart with the 90-day window", () => {
    render(<WorkerEscalationPage />);
    expect(screen.getByTestId("trust-over-time-chart")).toBeInTheDocument();
    expect(screen.getByTestId("trust-over-time-line")).toBeInTheDocument();
  });
});

describe("EscalationMatrix (unit)", () => {
  it("renders one routed row per rule and includes the channel hint", () => {
    render(
      <EscalationMatrix
        workerName="Mariah"
        rules={[
          {
            action: "Reply to buyer",
            condition: "Multiple competing offers",
            routesTo: "Seller",
            channel: "SMS",
          },
          {
            action: "Approve photos",
            condition: "Edited-photo signal fires",
            routesTo: "Owner",
          },
        ]}
      />,
    );
    const rows = screen.getAllByTestId("escalation-matrix-row");
    expect(rows).toHaveLength(2);
    expect(screen.getByText("via SMS")).toBeInTheDocument();
  });
});

describe("TrustOverTimeChart (unit)", () => {
  it("renders 90 points for a known worker", () => {
    expect(findSampleTrustHistory("wkr_01J0MARIAH")).toHaveLength(90);
  });

  it("shows an empty-state when the points array is empty", () => {
    render(<TrustOverTimeChart points={[]} workerName="Nobody" />);
    expect(screen.getByTestId("trust-over-time-empty")).toBeInTheDocument();
  });

  it("renders the line path when points are present", () => {
    const points = findSampleTrustHistory("wkr_01J0MARIAH");
    render(<TrustOverTimeChart points={points} workerName="Mariah" />);
    const line = screen.getByTestId("trust-over-time-line");
    const d = line.getAttribute("d") ?? "";
    expect(d.startsWith("M")).toBe(true);
    expect(d.split(" ").length).toBeGreaterThanOrEqual(90);
  });

  it("declares a prefers-reduced-motion guard so the reveal animation is suppressed", () => {
    const points = findSampleTrustHistory("wkr_01J0MARIAH");
    const { container } = render(
      <TrustOverTimeChart points={points} workerName="Mariah" />,
    );
    const styleEl = container.querySelector("style");
    const css = styleEl?.textContent ?? "";
    expect(css).toMatch(/prefers-reduced-motion/);
    expect(css).toMatch(/animation:\s*none/);
  });

  it("opens a tooltip with the date + grade label after a keyboard focus + ArrowRight press", async () => {
    const user = userEvent.setup();
    const points = findSampleTrustHistory("wkr_01J0MARIAH");
    render(<TrustOverTimeChart points={points} workerName="Mariah" />);
    const svg = screen.getByLabelText(/Trust grade over time for Mariah/);
    svg.focus();
    await user.keyboard("{ArrowRight}");
    const tooltip = await screen.findByTestId("trust-over-time-tooltip");
    expect(tooltip).toBeInTheDocument();
    expect(tooltip.getAttribute("data-grade")).toBeTruthy();
  });
});

describe("sample-worker-trust-history seeds", () => {
  it("are deterministic — reference date is fixed and every series ends inside the window", () => {
    expect(REFERENCE_NOW).toBe(Date.UTC(2026, 4, 21, 0, 0, 0));
    for (const [workerId, points] of Object.entries(SAMPLE_WORKER_TRUST_HISTORY)) {
      expect(workerId).toMatch(/^wkr_/);
      expect(points).toHaveLength(90);
      const lastTs = points.at(-1)!.ts;
      expect(lastTs).toBe(REFERENCE_NOW);
    }
  });

  it("maps numeric values back to canonical grade ids", () => {
    expect(gradeForValue(0)).toBe("learned_unreviewed");
    expect(gradeForValue(2)).toBe("tenant");
    expect(gradeForValue(4)).toBe("builtin");
    expect(gradeForValue(7)).toBe("builtin"); // clamped
    expect(gradeForValue(-1)).toBe("learned_unreviewed"); // clamped
  });
});
