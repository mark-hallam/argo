import { render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { AuditDayGroup } from "@/components/skycot/audit/AuditDayGroup";
import type { AuditDayBucket, AuditRow } from "@/lib/types/audit";

vi.mock("next/link", () => ({
  __esModule: true,
  default: ({
    children,
    prefetch: _prefetch,
    ...props
  }: { children: React.ReactNode; prefetch?: boolean }) => (
    <a {...props}>{children}</a>
  ),
}));

const pushMock = vi.fn();
vi.mock("next/navigation", () => ({
  useRouter: () => ({ push: pushMock }),
}));

vi.mock("@/lib/ui/use-toast", () => ({
  useToast: () => ({
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
    warning: vi.fn(),
    loading: vi.fn(),
    promise: vi.fn(),
    dismiss: vi.fn(),
  }),
}));

const ROW_A: AuditRow = {
  id: "act_001",
  timestamp: "2026-05-27T08:00:00Z",
  worker: { id: "wkr_mariah", name: "Mariah" },
  actionType: "message_sent",
  verb: "drafted reply to Acme renewal",
  status: "succeeded",
  selfHash: "0xabc123def456",
};

const ROW_B: AuditRow = {
  id: "act_002",
  timestamp: "2026-05-27T07:45:00Z",
  worker: { id: "wkr_peta", name: "Peta" },
  actionType: "approval_asked",
  verb: "asked Mark to approve refund",
  status: "pending",
  sampleHash: "0xfeedface00",
};

const BUCKET: AuditDayBucket = {
  key: "2026-05-27",
  label: "TODAY",
  rows: [ROW_A, ROW_B],
};

describe("AuditDayGroup", () => {
  it("renders the day bucket as a real <table> with thead/tbody/tr/th/td", () => {
    const { container } = render(<AuditDayGroup bucket={BUCKET} />);

    // Real <table> element (not div[role=list]).
    const table = container.querySelector("table");
    expect(table).not.toBeNull();
    expect(table?.getAttribute("data-testid")).toBe(
      `audit-day-table-${BUCKET.key}`,
    );

    // <caption> for screen readers.
    const caption = table?.querySelector("caption");
    expect(caption?.textContent).toContain(BUCKET.label);

    // <thead> with one row of 5 <th scope="col"> column headers.
    const thead = table?.querySelector("thead");
    expect(thead).not.toBeNull();
    const headerCells = thead?.querySelectorAll('th[scope="col"]') ?? [];
    expect(headerCells.length).toBe(5);
    const headerLabels = Array.from(headerCells).map((c) => c.textContent);
    expect(headerLabels).toEqual([
      "Time",
      "Worker",
      "Verb",
      "Id",
      "Status",
    ]);

    // <tbody> with one <tr> per row, each with 5 <td> cells.
    const tbody = table?.querySelector("tbody");
    expect(tbody).not.toBeNull();
    const bodyRows = tbody?.querySelectorAll("tr") ?? [];
    expect(bodyRows.length).toBe(BUCKET.rows.length);
    bodyRows.forEach((tr) => {
      const cells = tr.querySelectorAll("td");
      expect(cells.length).toBe(5);
    });
  });

  it("no longer uses div[role=list] for the rows", () => {
    const { container } = render(<AuditDayGroup bucket={BUCKET} />);
    expect(container.querySelector('[role="list"]')).toBeNull();
    expect(container.querySelector('[role="listitem"]')).toBeNull();
  });

  it("preserves the per-row data hooks the audit page consumes", () => {
    render(<AuditDayGroup bucket={BUCKET} />);
    const row = screen.getByTestId(`audit-row-${ROW_A.id}`);
    expect(row.getAttribute("data-status")).toBe(ROW_A.status);
    expect(row.getAttribute("data-action-type")).toBe(ROW_A.actionType);
    expect(row.getAttribute("data-worker-id")).toBe(ROW_A.worker.id);
    expect(row.tagName.toLowerCase()).toBe("tr");
    // Verb cell renders the verb text.
    expect(within(row).getByTestId(`activity-row-verb-${ROW_A.id}`)).toHaveTextContent(
      ROW_A.verb,
    );
  });

  it("uses aria-labelledby on the section pointing at the day heading", () => {
    const { container } = render(<AuditDayGroup bucket={BUCKET} />);
    const section = container.querySelector("section");
    const heading = container.querySelector("h2");
    expect(heading?.id).toBeTruthy();
    expect(section?.getAttribute("aria-labelledby")).toBe(heading?.id);
  });
});
