import { render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { VerticalRow } from "@/lib/api/verticals";

/**
 * Brand-consistency contract for /workflows/build.
 *
 * Closes scene-4-and-5 F19: the bare Tailwind header that slipped past
 * the D3 brand-consistency pass. The recipe pinned here matches the
 * sibling /workflows/templates surface (Audiowide eyebrow + gradient
 * hero strong + Inter sub + magazine vertical cards).
 *
 * `cssModules` is false in the vitest config so we can't measure
 * computed styles — we pin the static class hooks that page.css
 * targets instead. `useVerticals` is mocked to the resolved
 * mock-offline fallback so the data-driven picker renders its cards
 * (not the loading skeleton) under test.
 */

const verticalsMock = vi.hoisted(() => ({
  useVerticals: vi.fn(() => ({
    verticals: [] as VerticalRow[],
    source: "mock-offline" as "gateway" | "mock-offline" | "mock-bypass",
    isLoading: false,
    error: null as Error | null,
  })),
}));

vi.mock("@/lib/api/verticals", async () => {
  const actual =
    await vi.importActual<typeof import("@/lib/api/verticals")>(
      "@/lib/api/verticals",
    );
  return { ...actual, useVerticals: verticalsMock.useVerticals };
});

import WorkflowBuildEntryPage from "./page";

describe("WorkflowBuildEntryPage — D3 brand recipe", () => {
  beforeEach(() => {
    verticalsMock.useVerticals.mockReturnValue({
      verticals: [],
      source: "mock-offline",
      isLoading: false,
      error: null,
    });
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("renders the Audiowide eyebrow + gradient hero", () => {
    render(<WorkflowBuildEntryPage />);

    expect(
      screen.getByText(/BUILD A WORKFLOW · CHOOSE A VERTICAL/),
    ).toBeInTheDocument();

    const hero = screen.getByRole("heading", { level: 1 });
    expect(hero).toHaveClass("build-hero");
    expect(hero.querySelector("strong")?.textContent).toBe("blank canvas");
  });

  it("does not regress to the bare Tailwind header from before the brand pass", () => {
    render(<WorkflowBuildEntryPage />);
    const hero = screen.getByRole("heading", { level: 1 });
    expect(hero.className).not.toMatch(/text-3xl|text-4xl|font-semibold/);
  });

  it("offers a templates CTA + the 5 vertical magazine cards", () => {
    render(<WorkflowBuildEntryPage />);

    expect(screen.getByTestId("build-templates-cta")).toHaveAttribute(
      "href",
      "/workflows/templates",
    );

    const picker = screen.getByTestId("workflow-build-vertical-picker");
    expect(picker).toBeInTheDocument();

    for (const id of ["sales", "support", "gtm-engineer", "people-ops", "finance"]) {
      const card = screen.getByTestId(`workflow-build-vertical-${id}`);
      expect(card).toHaveClass("build-card");
      expect(card).toHaveAttribute(
        "href",
        `/workflows/${id}/untitled/build`,
      );
    }
  });

  it("promotes the first vertical to a lead magazine tile (cohort-D3 register)", () => {
    render(<WorkflowBuildEntryPage />);
    const lead = screen.getByTestId("workflow-build-vertical-sales");
    expect(lead.className).toMatch(/build-card--lead/);
  });
});
