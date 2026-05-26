import { describe, expect, it } from "vitest";
import { NAV_ITEMS } from "@/components/skycot/nav-items";

// Closes whole-site-mvp-claims-coverage F23 [P3] — the sidebar order
// is operator-canonical: Approvals first (the inbox surface), then the
// people surfaces (Team / Workers / Cohorts), then the catalogue
// surfaces (Skills / Workflows), with Audit last. Cohorts must sit in
// the people cluster, not at the end of the list (D3 design lock).

describe("sidebar nav order", () => {
  it("matches the operator-canonical sequence", () => {
    expect(NAV_ITEMS.map((item) => item.label)).toEqual([
      "Approvals",
      "Team",
      "Workers",
      "Cohorts",
      "Skills",
      "Workflows",
      "Audit",
    ]);
  });

  it("places Cohorts before Skills, Workflows and Audit", () => {
    const labels = NAV_ITEMS.map((item) => item.label);
    const cohorts = labels.indexOf("Cohorts");
    expect(cohorts).toBeGreaterThan(-1);
    expect(cohorts).toBeLessThan(labels.indexOf("Skills"));
    expect(cohorts).toBeLessThan(labels.indexOf("Workflows"));
    expect(cohorts).toBeLessThan(labels.indexOf("Audit"));
  });
});
