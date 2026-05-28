/**
 * Acceptance test for the demo-mode pick-vertical pattern.
 *
 * Pins:
 * - the honest-relabel from
 *   `fix-onboarding-pick-vertical-demo-mode-honest-install-relabel`
 *   (subtitle does NOT claim "We install ..." as a present-tense
 *   action in demo mode);
 * - the chip-clutter cleanup from
 *   `fix-onboarding-pick-vertical-demo-mode-chip-clutter` (no per-
 *   card demo chips, so the 7-card grid stops shouting it);
 * - the page-level demo banner dedupe from
 *   `fix-onboarding-per-step-demo-banner-dedupe-against-global-shell-banner`
 *   (the page-level banner is removed — the shell-wide
 *   `DemoModeBanner` mounted on `WizardEditorialShell` is the single
 *   demo-mode indicator now);
 * - the gateway-truth render from
 *   `fix-wizard-verticals-rewrite-scope-and-dead-fallback` (cards
 *   come from `useVerticals` only — the dead 7-entry
 *   `FALLBACK_VERTICALS` const has been deleted).
 */

import { render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { VerticalRow } from "@/lib/api/verticals";

const demoModeMock = vi.hoisted(() => ({
  isDemoMode: vi.fn((): boolean => false),
}));

/**
 * Minimal `VerticalRow` factory so each test can mock the gateway
 * response without redeclaring every field. Mirrors the shape the
 * canonical `/api/v1/verticals` endpoint returns post-#2456.
 */
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

const SEVEN_GATEWAY_VERTICALS: ReadonlyArray<VerticalRow> = [
  vrow("gtm-engineer", "GTM Engineer", 8),
  vrow("marketing-intern", "Marketing Intern", 5),
  vrow("marketplace-seller", "Marketplace Seller", 2),
  vrow("recruiter", "Recruiter", 5),
  vrow("sales-sdr", "Sales SDR", 5),
  vrow("customer-support", "Customer Support", 5),
  vrow("finance-ar", "Accounts Receivable", 5),
];

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

vi.mock("next/navigation", () => ({
  useRouter: () => ({
    push: vi.fn(),
    replace: vi.fn(),
    back: vi.fn(),
  }),
  // Required by WizardEditorialShell (mounted via OnboardingShell).
  usePathname: () => "/onboarding/pick-vertical",
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

describe("onboarding pick-vertical demo-mode relabel", () => {
  beforeEach(() => {
    demoModeMock.isDemoMode.mockReset();
    verticalsMock.useVerticals.mockReset();
    verticalsMock.useVerticals.mockReturnValue({
      verticals: [...SEVEN_GATEWAY_VERTICALS],
      source: "gateway" as const,
      isLoading: false,
      error: null,
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("ships the real-install subtitle outside demo mode", () => {
    demoModeMock.isDemoMode.mockReturnValue(false);

    render(<OnboardingPickVerticalPage />);

    expect(
      screen.getByTestId("onboarding-vertical-subtitle-real"),
    ).toHaveTextContent("We install the");
    expect(
      screen.queryByTestId("onboarding-vertical-subtitle-demo"),
    ).not.toBeInTheDocument();
    // No demo banner and no per-card demo chip in real mode.
    expect(
      screen.queryByTestId("pick-vertical-demo-banner"),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByTestId("vertical-gtm-engineer-demo-chip"),
    ).not.toBeInTheDocument();
  });

  it("ships the honest demo-mode subtitle when NEXT_PUBLIC_SKYCOT_DEMO_MODE=1", () => {
    demoModeMock.isDemoMode.mockReturnValue(true);

    render(<OnboardingPickVerticalPage />);

    const demoSub = screen.getByTestId("onboarding-vertical-subtitle-demo");
    expect(demoSub).toHaveTextContent(/Demo mode/i);
    expect(demoSub.textContent ?? "").toMatch(
      /once you create a real workspace/i,
    );
    // Demo-mode subtitle must NOT claim install is happening present-tense.
    expect(demoSub.textContent ?? "").not.toMatch(/^We install /);
    expect(
      screen.queryByTestId("onboarding-vertical-subtitle-real"),
    ).not.toBeInTheDocument();
  });

  it("renders NO page-level demo banner and NO per-card demo chips (shell banner dedupe)", () => {
    demoModeMock.isDemoMode.mockReturnValue(true);

    render(<OnboardingPickVerticalPage />);

    // The page-level banner that previously sat below the back-CTA row
    // has been removed. Demo-mode framing is now carried by the
    // shell-wide `DemoModeBanner` mounted on `WizardEditorialShell`
    // (F15 dedupe); the page must not render its own competing banner.
    expect(
      screen.queryByTestId("pick-vertical-demo-banner"),
    ).not.toBeInTheDocument();

    // None of the demo-supported tiles carry the old per-card chip.
    // (The 3 unmapped fallback tiles are also filtered out in demo
    // mode per F10; see the grid-restriction test below.)
    const renderedTiles = [
      "marketing-intern",
      "marketplace-seller",
      "recruiter",
      "customer-support",
    ];
    for (const id of renderedTiles) {
      expect(
        screen.queryByTestId(`vertical-${id}-demo-chip`),
      ).not.toBeInTheDocument();
    }
  });

  it("F10 — restricts demo grid to the 4 mapped verticals so picks never fall through to Mariah", () => {
    demoModeMock.isDemoMode.mockReturnValue(true);

    render(<OnboardingPickVerticalPage />);

    // The 4 verticals the demo first-hire short-circuit maps to a real
    // sample persona must render. Picking any of these lands on a
    // /workers/<id> whose persona matches the vertical the user chose.
    const supported = [
      "marketplace-seller",
      "recruiter",
      "customer-support",
      "marketing-intern",
    ];
    for (const id of supported) {
      expect(screen.queryByTestId(`vertical-${id}`)).toBeInTheDocument();
    }

    // The 3 unmapped fallback verticals previously fell through to
    // `DEMO_FALLBACK_WORKER = Mariah`, rendering Mariah's marketplace-
    // lister charter no matter which one the user picked. They must
    // not render in demo mode.
    const unmapped = ["gtm-engineer", "sales-sdr", "finance-ar"];
    for (const id of unmapped) {
      expect(screen.queryByTestId(`vertical-${id}`)).not.toBeInTheDocument();
    }

    // Honest count line — "Demo · N verticals available" — replaces
    // the "v1 catalogue · 7 verticals" copy so the operator isn't
    // told the full catalogue is reachable from the demo build.
    const sourceLine = screen.getByTestId("onboarding-vertical-source");
    expect(sourceLine.textContent ?? "").toMatch(/Demo · 4 verticals available/);
  });

  it("renders every gateway-loaded vertical outside demo mode (filter is demo-only)", () => {
    demoModeMock.isDemoMode.mockReturnValue(false);

    render(<OnboardingPickVerticalPage />);

    const all = [
      "gtm-engineer",
      "marketing-intern",
      "marketplace-seller",
      "recruiter",
      "sales-sdr",
      "customer-support",
      "finance-ar",
    ];
    for (const id of all) {
      expect(screen.queryByTestId(`vertical-${id}`)).toBeInTheDocument();
    }
  });

  it("renders one card per gateway row (gateway-truth pin, no FALLBACK)", () => {
    // Brief acceptance: "Vitest pin: when `useVerticals` returns N
    // rows, the page renders N cards." Pick a number larger than the
    // old 7-entry hardcoded fallback to prove cards track the gateway
    // catalogue rather than a baked-in constant.
    demoModeMock.isDemoMode.mockReturnValue(false);
    const thirty: VerticalRow[] = Array.from({ length: 30 }, (_, i) =>
      vrow(`pack-${i}`, `Pack ${i}`, i),
    );
    verticalsMock.useVerticals.mockReturnValue({
      verticals: thirty,
      source: "gateway" as const,
      isLoading: false,
      error: null,
    });

    render(<OnboardingPickVerticalPage />);

    // Only the 30 vertical cards carry `aria-pressed`; the back
    // button doesn't, so the role-query bounds the card count tightly.
    expect(screen.getAllByRole("button", { pressed: false })).toHaveLength(30);
    // First and last gateway-supplied cards both render.
    expect(screen.queryByTestId("vertical-pack-0")).toBeInTheDocument();
    expect(screen.queryByTestId("vertical-pack-29")).toBeInTheDocument();
    // Source line surfaces the gateway count, not a baked-in number.
    expect(
      screen.getByTestId("onboarding-vertical-source").textContent ?? "",
    ).toMatch(/v1 catalogue · 30 verticals/);
  });

  it("renders zero cards when the gateway returns an empty catalogue (no FALLBACK)", () => {
    // The dead 7-entry FALLBACK_VERTICALS const used to mask gateway
    // failures behind a baked-in grid. After deletion, an empty
    // gateway response renders an empty grid — surfacing the bug to
    // the operator instead of hiding it.
    demoModeMock.isDemoMode.mockReturnValue(false);
    verticalsMock.useVerticals.mockReturnValue({
      verticals: [],
      source: "gateway" as const,
      isLoading: false,
      error: null,
    });

    render(<OnboardingPickVerticalPage />);

    expect(
      screen.queryByTestId("vertical-gtm-engineer"),
    ).not.toBeInTheDocument();
    expect(screen.queryByTestId("vertical-recruiter")).not.toBeInTheDocument();
    expect(
      screen.getByTestId("onboarding-vertical-source").textContent ?? "",
    ).toMatch(/v1 catalogue · 0 verticals/);
  });
});
