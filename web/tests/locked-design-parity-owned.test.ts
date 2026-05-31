import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

const file = (relativePath: string) =>
  fs.readFileSync(path.join(process.cwd(), relativePath), "utf8");

const HIGH_IMPACT_TOKEN_FILES = [
  "app/(app)/settings/layout.tsx",
  "app/(app)/cohorts/page.tsx",
  "components/skycot/cohorts/CohortMagazineStyles.tsx",
  "app/(app)/approvals/page.tsx",
  "app/(app)/skills/page.css",
  "app/(app)/skills/[skillId]/page.css",
  "app/(app)/workflows/page.tsx",
  "app/(app)/workflows/build/page.tsx",
  "app/(app)/workflows/templates/page.tsx",
  "app/(app)/workflows/templates/layout.tsx",
  "app/(app)/workers/[workerId]/overview/page.tsx",
  "components/skycot/worker/WorkerPersonalityCard.tsx",
  "app/auth/signup/page.tsx",
] as const;

const BRAND_LITERAL_DEBT =
  /#(?:8b5cf6|ec4899|ff6b6b|14b8a6|1e1b4b|3f3f46|52525b|a1a1aa|ece9f5|fafaff|fbbf24|6d28d9|047857|b45309|b91c1c|991b1b)\b|rgba\(/i;

describe("owned locked-design parity", () => {
  it("keeps settings on the locked D top-tabs layout", () => {
    const src = file("app/(app)/settings/layout.tsx");
    expect(src).toContain("SettingsTabs");
    expect(src).toContain("settings-content-grid");
    expect(src).toContain("settings-aside");
  });

  it("keeps cohorts on the locked D3 magazine-tile recipe", () => {
    const page = file("app/(app)/cohorts/page.tsx");
    const styles = file("components/skycot/cohorts/CohortMagazineStyles.tsx");
    expect(page).toContain("CohortDeptTabs");
    expect(page).toContain("CohortMagazineGrid");
    expect(styles).toContain("skycot-cohort-mag-row1");
    expect(styles).toContain("skycot-cohort-lead-tile");
    expect(styles).toContain("--skycot-cohort-gradient: var(--brand-gradient);");
  });

  it("keeps approvals on the locked E ops-dashboard recipe", () => {
    const page = file("app/(app)/approvals/page.tsx");
    const dashboard = file("components/skycot/approvals/ApprovalsOpsDashboard.tsx");
    expect(page).toContain("ApprovalsOpsDashboard");
    expect(page).toContain('eyebrowDotColor="var(--warning)"');
    expect(dashboard).toContain('data-testid="approvals-ops-dashboard"');
    expect(dashboard).toContain("ApprovalsKpiStrip");
    expect(dashboard).toContain("ConfidenceDonut");
    expect(dashboard).toContain("SeveritySection");
  });

  it("keeps skills on the locked A1 marketplace recipe", () => {
    const page = file("app/(app)/skills/page.tsx");
    const css = file("app/(app)/skills/page.css");
    expect(page).toContain("Skills marketplace landing");
    expect(page).toContain("TRY_CHIPS");
    expect(page).toContain('data-testid="skills-search"');
    expect(page).toContain("FEATURED SKILLS");
    expect(css).toContain("featured-grid");
    expect(css).toContain("var(--brand-gradient)");
  });

  it("keeps onboarding on the locked F1 editorial-center shell", () => {
    const shell = file("app/(setup)/onboarding/_OnboardingShell.tsx");
    const wizard = file("components/auth/WizardEditorialShell.tsx");
    expect(shell).toContain("WizardEditorialShell");
    expect(shell).toContain("forceExitConfirm");
    expect(shell).toContain("OnboardingFormStyles");
    expect(wizard).toContain("Locked F1 editorial-center wizard shell");
    expect(wizard).toContain("wiz-editorial");
  });

  it("keeps worker detail on the locked C2 personality recipe", () => {
    const overview = file("app/(app)/workers/[workerId]/overview/page.tsx");
    const card = file("components/skycot/worker/WorkerPersonalityCard.tsx");
    expect(overview).toContain("WorkerPersonalityCard");
    expect(card).toContain("Worker personality card");
    expect(card).toContain("wpc-chips");
    expect(card).toContain("SIGNATURE BEHAVIORS");
  });

  it("keeps high-impact owned surfaces on runtime brand tokens", () => {
    const failures = HIGH_IMPACT_TOKEN_FILES.flatMap((relativePath) => {
      const src = file(relativePath);
      return BRAND_LITERAL_DEBT.test(src) ? [relativePath] : [];
    });
    expect(failures).toEqual([]);
  });
});
