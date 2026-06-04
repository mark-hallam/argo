import { render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { WorkflowSummary } from "@/lib/sample-workflows";
import type { WorkflowDataSource } from "@/lib/api/workflows";

/**
 * WS2 — fresh-tenant empty state for `/workflows`.
 *
 * Pins that a brand-new workspace (live fetch returns zero workflows,
 * no search query) renders the truth-bounded "No workflows yet" panel
 * with the single CTA routing to the template library — NOT the
 * search-empty "No workflows match" panel, and NOT a seeded grid.
 */

type WorkflowsQueryShape = {
  data: { workflows: WorkflowSummary[]; source: WorkflowDataSource } | undefined;
  isLoading: boolean;
  isError: boolean;
};

const useWorkflowsMock = vi.fn<() => WorkflowsQueryShape>();

vi.mock("@/lib/hooks/use-workflows", () => ({
  useWorkflows: () => useWorkflowsMock(),
}));

// The catalog grid's `WorkflowAuthorChip` calls `useRouter()`; provide a
// stub so the populated-grid assertion can render a card.
vi.mock("next/navigation", () => ({
  useRouter: () => ({
    push: vi.fn(),
    replace: vi.fn(),
    prefetch: vi.fn(),
    refresh: vi.fn(),
    back: vi.fn(),
    forward: vi.fn(),
  }),
  usePathname: () => "/workflows",
  useSearchParams: () => new URLSearchParams(),
}));

// The propose dialog drags in react-query (useTeam) + toast; the empty
// state under test never opens it, so stub it to nothing.
vi.mock("@/components/skycot/WorkflowProposeModal", () => ({
  WorkflowProposeModal: () => null,
}));

// Tooltip wrapper around the "Propose new" button — passthrough so the
// button still renders without a Radix provider in the test tree.
vi.mock("@/components/ui/Tooltip", () => ({
  HoverHint: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

import WorkflowsPage from "../page";

const SAMPLE: WorkflowSummary[] = [
  {
    name: "approve-refund",
    vertical: "support",
    description: "Route refund approvals to a human.",
    risk: "approval-required",
    steps: 4,
    versions: 2,
    lastEdited: "2d ago",
    lastEditor: "Mariah",
    authorType: "worker",
  } as unknown as WorkflowSummary,
];

beforeEach(() => {
  useWorkflowsMock.mockReset();
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("/workflows fresh-tenant empty state (WS2)", () => {
  it("renders the fresh empty panel when the live list is zero rows and no query", () => {
    useWorkflowsMock.mockReturnValue({
      data: { workflows: [], source: "gateway" },
      isLoading: false,
      isError: false,
    });
    render(<WorkflowsPage />);

    const empty = screen.getByTestId("workflows-fresh-empty");
    expect(empty).toBeInTheDocument();
    expect(screen.getByText(/no workflows yet\./i)).toBeInTheDocument();
    // Truth-bounded, future-framed copy — no fabricated counts/data.
    expect(
      screen.getByText(/plain-english playbooks your teammates follow/i),
    ).toBeInTheDocument();
    // The single CTA routes forward to the template library.
    expect(screen.getByTestId("workflows-fresh-empty-cta")).toHaveAttribute(
      "href",
      "/workflows/templates",
    );
    // Not the search-empty panel.
    expect(screen.queryByTestId("workflows-empty")).not.toBeInTheDocument();
  });

  it("renders the catalog grid (not the fresh empty) when workflows exist", () => {
    useWorkflowsMock.mockReturnValue({
      data: { workflows: SAMPLE, source: "gateway" },
      isLoading: false,
      isError: false,
    });
    render(<WorkflowsPage />);

    expect(screen.queryByTestId("workflows-fresh-empty")).not.toBeInTheDocument();
    expect(screen.getByText("approve-refund")).toBeInTheDocument();
  });
});
