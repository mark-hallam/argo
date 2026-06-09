/**
 * Acceptance test for `mission-genesis-onboarding-pick-vertical-add-describe-own`.
 *
 * Pins the WS3 / Genesis hire-path CTA:
 * - the `<GenesisDescribeCta>` unit renders the
 *   `onboarding-genesis-describe-cta` element and `router.push("/genesis")`
 *   on click (Genesis is the describe-and-watch headline path);
 * - the onboarding step-04 page renders that CTA ABOVE the still-present
 *   `onboarding-vertical-grid` template grid (templates stay as the
 *   fast-lane; the grid is unchanged).
 */

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { VerticalRow } from "@/lib/api/verticals";
import { GenesisDescribeCta } from "@/components/skycot/onboarding/pick-vertical/GenesisDescribeCta";

const pushMock = vi.hoisted(() => vi.fn());

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push: pushMock, replace: vi.fn(), back: vi.fn() }),
  // Required by WizardEditorialShell (mounted via OnboardingShell).
  usePathname: () => "/onboarding/pick-vertical",
}));

const demoModeMock = vi.hoisted(() => ({
  isDemoMode: vi.fn((): boolean => false),
}));

function vrow(id: string, name: string, skill_count = 5): VerticalRow {
  return {
    id,
    name,
    version: "0.1.0",
    description: `${name} description.`,
    headline: `${name} headline.`,
    tags: [],
    workflow_count: 0,
    action_count: 0,
    role_label: name,
    kpi_count: 0,
    skill_count,
  };
}

const verticalsMock = vi.hoisted(() => ({
  useVerticals: vi.fn(() => ({
    verticals: [] as VerticalRow[],
    source: "gateway" as const,
    isLoading: false,
    error: null,
  })),
}));

const draftMock = vi.hoisted(() => {
  const state = {
    verticalId: null as string | null,
    hasHydrated: true,
    setStep: vi.fn(),
    setVertical: vi.fn(),
  };
  type Selector = (s: typeof state) => unknown;
  const hook = vi.fn((selector: Selector) => selector(state));
  return { state, hook };
});

const apiClientMock = vi.hoisted(() => ({
  apiPost: vi.fn(),
  ApiError: class ApiError extends Error {
    status = 0;
  },
}));

vi.mock("@/lib/skycot/demo-mode", () => demoModeMock);
vi.mock("@/lib/api/verticals", async () => {
  const actual =
    await vi.importActual<typeof import("@/lib/api/verticals")>(
      "@/lib/api/verticals",
    );
  return { ...actual, useVerticals: verticalsMock.useVerticals };
});
vi.mock("@/lib/onboarding/use-onboarding-draft", () => ({
  useOnboardingDraft: draftMock.hook,
}));
vi.mock("@/lib/api/client", () => apiClientMock);

import OnboardingPickVerticalPage from "../page";

describe("GenesisDescribeCta unit", () => {
  beforeEach(() => {
    pushMock.mockClear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("renders the describe-your-own CTA with the canonical testid", () => {
    render(<GenesisDescribeCta />);
    const cta = screen.getByTestId("onboarding-genesis-describe-cta");
    expect(cta).toBeInTheDocument();
    expect(cta).toHaveTextContent(/Describe your own teammate/i);
    expect(cta).toHaveTextContent(/plain English/i);
  });

  it("routes to /genesis on click", async () => {
    const user = userEvent.setup();
    render(<GenesisDescribeCta />);
    await user.click(screen.getByTestId("onboarding-genesis-describe-cta"));
    expect(pushMock).toHaveBeenCalledWith("/genesis");
  });
});

describe("onboarding pick-vertical Genesis entry", () => {
  beforeEach(() => {
    pushMock.mockClear();
    demoModeMock.isDemoMode.mockReset();
    demoModeMock.isDemoMode.mockReturnValue(false);
    verticalsMock.useVerticals.mockReset();
    verticalsMock.useVerticals.mockReturnValue({
      verticals: [
        vrow("gtm-engineer", "GTM Engineer", 8),
        vrow("recruiter", "Recruiter", 5),
      ],
      source: "gateway" as const,
      isLoading: false,
      error: null,
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("renders the Genesis CTA above the still-present template grid", () => {
    render(<OnboardingPickVerticalPage />);

    const cta = screen.getByTestId("onboarding-genesis-describe-cta");
    const grid = screen.getByTestId("onboarding-vertical-grid");

    // Both the headline Genesis path and the canned-template fast-lane
    // are present...
    expect(cta).toBeInTheDocument();
    expect(grid).toBeInTheDocument();
    expect(screen.getByTestId("vertical-gtm-engineer")).toBeInTheDocument();

    // ...and the CTA precedes the grid in document order (rendered ABOVE).
    expect(
      cta.compareDocumentPosition(grid) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
  });

  it("CTA routes to /genesis from inside the wizard", async () => {
    const user = userEvent.setup();
    render(<OnboardingPickVerticalPage />);
    await user.click(screen.getByTestId("onboarding-genesis-describe-cta"));
    expect(pushMock).toHaveBeenCalledWith("/genesis");
  });
});
