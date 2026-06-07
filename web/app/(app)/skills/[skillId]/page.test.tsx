import { fireEvent, render, screen } from "@testing-library/react";
import { Suspense } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

// Mock the skills hooks so we can drive query states directly. The
// page's only load-bearing query for the error branch is
// `useSkillUsage`; telemetry/versions return null (honest empty).
const useSkillUsageMock = vi.fn();
const refetchMock = vi.fn();

vi.mock("@/lib/hooks/use-skills", () => ({
  useSkillUsage: () => useSkillUsageMock(),
  useSkillTelemetry: () => ({ data: { telemetry: null } }),
  useSkillVersions: () => ({ data: { versions: null } }),
}));

// Stub the heavy card subcomponents — they're exercised by their own
// suites; here we only care about the page's error vs zero-state branch.
vi.mock("@/components/skycot/skills/SkillLearningLoopCard", () => ({
  SkillLearningLoopCard: () => <div data-testid="learning-loop-card" />,
}));
vi.mock("@/components/skycot/skills/SkillVersionHistoryCard", () => ({
  SkillVersionHistoryCard: () => <div data-testid="version-history-card" />,
}));
vi.mock("@/components/skycot/skills/InstallSkillButton", () => ({
  InstallSkillButton: () => <button type="button">Install</button>,
}));
vi.mock("@/components/skycot/skills/TrustGradeChip", () => ({
  TrustGradeChip: () => <span data-testid="trust-grade-chip" />,
}));

import SkillDetailStubPage from "./page";

// React's `use(promise)` reads a thenable synchronously when it
// carries the fulfilled-thenable shape (`status`/`value`), instead of
// suspending. We hand the page such a pre-settled params promise so the
// committed render is available immediately — no Suspense round-trip to
// race against happy-dom's microtask flushing.
function settledParams(skillId: string): Promise<{ skillId: string }> {
  const p = Promise.resolve({ skillId }) as Promise<{ skillId: string }> & {
    status?: string;
    value?: { skillId: string };
  };
  p.status = "fulfilled";
  p.value = { skillId };
  return p;
}

function renderPage() {
  return render(
    <Suspense fallback={<div data-testid="page-suspense-fallback" />}>
      <SkillDetailStubPage params={settledParams("approval.route")} />
    </Suspense>,
  );
}

beforeEach(() => {
  useSkillUsageMock.mockReset();
  refetchMock.mockReset();
});

describe("/skills/[skillId] detail page", () => {
  it("renders an error+retry card on a failed usage fetch, not the all-zeros default", async () => {
    useSkillUsageMock.mockReturnValue({
      data: undefined,
      isPending: false,
      isError: true,
      error: new Error("gateway unreachable"),
      refetch: refetchMock,
    });

    renderPage();

    // Error card present.
    expect(screen.getByTestId("skill-detail-error")).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: /couldn’t load this skill/i }),
    ).toBeInTheDocument();

    // The fabricated zero-state must NOT render on error.
    expect(screen.queryByTestId("skill-provenance")).not.toBeInTheDocument();
    expect(screen.queryByTestId("skill-adoption-count")).not.toBeInTheDocument();
    expect(screen.queryByText(/authored by your org/i)).not.toBeInTheDocument();

    // Retry is wired to refetch.
    fireEvent.click(screen.getByRole("button", { name: /try again/i }));
    expect(refetchMock).toHaveBeenCalledTimes(1);
  });

  it("renders the provenance + adoption cards on a successful (empty) fetch", async () => {
    useSkillUsageMock.mockReturnValue({
      data: {
        usage: {
          skill_id: "00000000-0000-0000-0000-000000000001",
          skill_name: "approval.route",
          author_label: "Authored by SKYCOT",
          trust_grade: "seed",
          workers_using: [],
          adoption_count: 0,
        },
      },
      isPending: false,
      isError: false,
      refetch: refetchMock,
    });

    renderPage();

    expect(screen.getByTestId("skill-provenance")).toBeInTheDocument();
    expect(screen.getByTestId("skill-adoption-count")).toBeInTheDocument();
    expect(screen.queryByTestId("skill-detail-error")).not.toBeInTheDocument();
  });
});
