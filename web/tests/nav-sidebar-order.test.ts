import { describe, expect, it } from "vitest";
import { NAV_ITEMS } from "@/components/skycot/nav-items";

// Closes whole-site-mvp-claims-coverage F23 [P3] — the sidebar order
// is operator-canonical: Approvals first (the inbox surface), then the
// people surfaces (Team / Workers / Genesis / Cohorts), then the
// catalogue surfaces (Skills / Workflows), with Audit last. Cohorts
// must sit in the people cluster, not at the end of the list (D3
// design lock). Genesis sits between Workers and Cohorts so the
// "hire" surface is one click from any people-surface page — it is
// the investor-locked Onboarding Studio entry point and was previously
// orphaned (audit `2026-05-26-2309/onboarding-hire-an-ai-teammate.md`
// F1).

describe("sidebar nav order", () => {
  it("matches the operator-canonical sequence", () => {
    expect(NAV_ITEMS.map((item) => item.label)).toEqual([
      "Approvals",
      "Team",
      "Workers",
      "Genesis",
      "Pitch",
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

  it("exposes Genesis between Workers and Cohorts", () => {
    const labels = NAV_ITEMS.map((item) => item.label);
    const genesis = labels.indexOf("Genesis");
    expect(genesis).toBeGreaterThan(-1);
    expect(genesis).toBe(labels.indexOf("Workers") + 1);
    expect(genesis).toBeLessThan(labels.indexOf("Cohorts"));
  });
});
