import { render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import {
  WORKER_TAB_IDS,
  WORKER_TAB_LABELS,
  WorkerTabs,
  activeWorkerTab,
  isWorkerTabId,
} from "./WorkerTabs";

vi.mock("next/navigation", () => ({
  usePathname: () => "/workers/wkr_mariah/schedules",
}));

const WORKER_ID = "wkr_mariah";

describe("WorkerTabs composition", () => {
  it("places 'schedules' between 'overview' and 'activity'", () => {
    const overviewIdx = WORKER_TAB_IDS.indexOf("overview");
    const schedulesIdx = WORKER_TAB_IDS.indexOf("schedules");
    const activityIdx = WORKER_TAB_IDS.indexOf("activity");
    expect(overviewIdx).toBeGreaterThanOrEqual(0);
    expect(activityIdx).toBeGreaterThan(overviewIdx);
    expect(schedulesIdx).toBe(overviewIdx + 1);
    expect(schedulesIdx).toBe(activityIdx - 1);
  });

  it("labels the schedules tab 'Schedules'", () => {
    expect(WORKER_TAB_LABELS.schedules).toBe("Schedules");
  });

  it("treats 'schedules' as a valid WorkerTabId", () => {
    expect(isWorkerTabId("schedules")).toBe(true);
  });

  it("recognises /workers/<id>/schedules as the schedules tab", () => {
    expect(activeWorkerTab("/workers/wkr_mariah/schedules")).toBe("schedules");
  });
});

describe("WorkerTabs render", () => {
  it("renders a Schedules link pointing at /workers/<id>/schedules", () => {
    render(<WorkerTabs workerId={WORKER_ID} />);
    const link = screen.getByTestId("worker-tab-schedules");
    expect(link).toHaveAttribute("href", `/workers/${WORKER_ID}/schedules`);
    expect(link).toHaveTextContent("Schedules");
  });

  it("renders the tab strip in the canonical order with Schedules between Overview and Activity", () => {
    render(<WorkerTabs workerId={WORKER_ID} />);
    const nav = screen.getByTestId("worker-tabs");
    const items = within(nav).getAllByRole("tab");
    const order = items.map((el) => el.textContent?.trim());
    const overviewIdx = order.indexOf("Overview");
    const schedulesIdx = order.indexOf("Schedules");
    const activityIdx = order.indexOf("Activity");
    expect(overviewIdx).toBeGreaterThanOrEqual(0);
    expect(schedulesIdx).toBe(overviewIdx + 1);
    expect(activityIdx).toBe(schedulesIdx + 1);
  });

  it("marks the Schedules tab as active when the pathname matches", () => {
    render(<WorkerTabs workerId={WORKER_ID} />);
    const link = screen.getByTestId("worker-tab-schedules");
    expect(link).toHaveAttribute("aria-selected", "true");
    expect(link).toHaveAttribute("data-active", "true");
  });
});
