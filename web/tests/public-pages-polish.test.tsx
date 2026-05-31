import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

const mockState = vi.hoisted(() => ({
  pathname: "/landing",
  setSelectedPlan: vi.fn(),
}));

vi.mock("next/navigation", () => ({
  usePathname: () => mockState.pathname,
  useSearchParams: () => new URLSearchParams(),
}));

vi.mock("@/lib/skycot/use-session", () => ({
  useSkycotSession: () => ({
    status: "unauthenticated",
    isDemoMode: false,
    email: null,
    displayName: "Guest",
    tenantId: null,
    workspaceLabel: "Personal workspace",
    role: null,
  }),
}));

vi.mock("@/lib/onboarding/use-onboarding-draft", () => ({
  useOnboardingDraft: (selector: (state: { setSelectedPlan: typeof mockState.setSelectedPlan }) => unknown) =>
    selector({ setSelectedPlan: mockState.setSelectedPlan }),
}));

import AboutLayout from "@/app/about/layout";
import AboutPage from "@/app/about/page";
import { CompareMatrix, type Dimension } from "@/app/compare/CompareMatrix";
import PricingPage from "@/app/pricing/page";
import SecurityLayout from "@/app/security/layout";
import SecurityPage from "@/app/security/page";

const THIS_DIR = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = join(THIS_DIR, "..", "..");
const WEB_ROOT = join(REPO_ROOT, "web");

function readWeb(path: string): string {
  return readFileSync(join(WEB_ROOT, path), "utf8");
}

describe("public page polish", () => {
  it("keeps customer and vertical public claims truth-bound", () => {
    const customerLayout = readWeb("app/customers/layout.tsx");
    const customerOg = readWeb("app/customers/opengraph-image.tsx");
    const verticalShell = readWeb("app/for/_components/VerticalLandingShell.tsx");

    const publicCopy = [customerLayout, customerOg, verticalShell].join("\n");
    expect(publicCopy).not.toMatch(/300\+ teams|Fortune 500|2,847 teams|WHAT TEAMS SAY/i);
    expect(customerLayout).toMatch(/Representative SKYCOT deployment profiles/);
    expect(customerOg).toMatch(/representative deployment profiles/i);
    expect(verticalShell).toMatch(/Representative profile/);
    expect(verticalShell).toMatch(/Hash-chained audit log/);
  });

  it("renders about and security with one main landmark from their layouts", () => {
    mockState.pathname = "/about";
    const about = render(
      <AboutLayout>
        <AboutPage />
      </AboutLayout>,
    );
    expect(about.container.querySelectorAll("main")).toHaveLength(1);
    expect(about.container.querySelectorAll("article")).toHaveLength(1);
    expect(screen.getByRole("heading", { name: /what makes skycot different/i })).toBeInTheDocument();
    about.unmount();

    mockState.pathname = "/security";
    const security = render(
      <SecurityLayout>
        <SecurityPage />
      </SecurityLayout>,
    );
    expect(security.container.querySelectorAll("main")).toHaveLength(1);
    expect(security.container.querySelectorAll("article")).toHaveLength(1);
    expect(screen.getByText(/external soc 2 type ii attestation is not claimed/i)).toBeInTheDocument();
  });

  it("labels the compare matrix table, rows, columns, and score glyphs", () => {
    const dimensions: Dimension[] = [
      {
        group: "Trust",
        question: "Can the teammate prove what happened?",
        scores: { SKYCOT: "yes", Zapier: "partial" },
        caption: "Hash-chained audit trail.",
      },
    ];

    render(<CompareMatrix competitors={["SKYCOT", "Zapier"]} dimensions={dimensions} />);

    const table = screen.getByRole("table", { name: /competitor comparison/i });
    expect(table).toBeInTheDocument();
    expect(within(table).getByRole("columnheader", { name: "Capability" })).toBeInTheDocument();
    expect(within(table).getByRole("rowheader", { name: /Trust/i })).toBeInTheDocument();
    expect(within(table).getByRole("rowheader", { name: /Can the teammate prove/i })).toBeInTheDocument();
    expect(within(table).getByRole("img", { name: "Yes" })).toBeInTheDocument();
    expect(within(table).getByRole("img", { name: "Partial" })).toBeInTheDocument();
  });

  it("renders pricing matrix and FAQ controls with accessible labels", () => {
    mockState.pathname = "/pricing";
    render(<PricingPage />);

    const table = screen.getByRole("table", { name: /pricing feature availability/i });
    expect(within(table).getByRole("columnheader", { name: "Enterprise" })).toBeInTheDocument();
    expect(within(table).getAllByRole("img", { name: "Included" }).length).toBeGreaterThan(0);

    const faqButton = screen.getByRole("button", { name: /per-teammate billing/i });
    expect(faqButton).toHaveAttribute("aria-expanded", "true");
    const controls = faqButton.getAttribute("aria-controls");
    expect(controls).toBeTruthy();
    expect(document.getElementById(controls!)).toHaveTextContent(/every named teammate/i);
  });

  it("uses runtime brand tokens on the touched public CSS surfaces", () => {
    const sources = [
      "components/marketing/marketing-nav.css",
      "app/landing/page.css",
      "app/customers/page.css",
      "app/customers/[slug]/page.css",
      "app/compare/page.css",
      "app/pricing/page.css",
      "app/for/_components/VerticalLandingShell.tsx",
    ];
    const forbidden = [
      /#8b5cf6/i,
      /#ec4899/i,
      /#ff6b6b/i,
      /#14b8a6/i,
      /rgba\(139,\s*92,\s*246/i,
      /rgba\(236,\s*72,\s*153/i,
      /linear-gradient\(135deg,\s*var\(--brand-primary\),\s*var\(--brand-teal\)/i,
    ];

    for (const source of sources) {
      const text = readWeb(source);
      for (const pattern of forbidden) {
        expect(text, `${source} contains deprecated or literal brand color ${pattern}`).not.toMatch(pattern);
      }
    }
  });

  it("keeps touched public CSS responsive and theme-tokened", () => {
    const sources = [
      "components/marketing/marketing-nav.css",
      "app/landing/page.css",
      "app/customers/page.css",
      "app/customers/[slug]/page.css",
      "app/compare/page.css",
      "app/pricing/page.css",
      "app/for/_components/VerticalLandingShell.tsx",
    ];

    for (const source of sources) {
      const text = readWeb(source);
      expect(text, `${source} needs a mobile/tablet breakpoint`).toMatch(/@media\s*\(max-width:/);
      expect(text, `${source} should inherit app theme surfaces`).toMatch(
        /var\(--background\)|var\(--foreground\)|var\(--card\)|var\(--border\)/,
      );
      expect(text, `${source} should not hard-code white UI surfaces`).not.toMatch(
        /(?:background|color):\s*(?:white|#fff|#ffffff)\b/i,
      );
    }
  });
});
