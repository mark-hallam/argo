import { describe, expect, it } from "vitest";
import { NAV_ITEMS } from "@/components/skycot/nav-items";

// Closes whole-site-mvp-claims-coverage F23 [P3] — the sidebar order
// is operator-canonical: Approvals first (the inbox surface), then the
// people surfaces (Team / Genesis / Cohorts), then the catalogue
// surfaces (Skills / Workflows), with Audit last. Cohorts must sit in
// the people cluster, not at the end of the list (D3 design lock).
// Genesis sits immediately after Team so the "hire" surface is one
// click from any people-surface page — it is the investor-locked
// Onboarding Studio entry point and was previously orphaned (audit
// `2026-05-26-2309/onboarding-hire-an-ai-teammate.md` F1).
//
// The roster has exactly ONE sidebar entry: "Team". The duplicate
// "Workers" entry was removed (fix-nav-dedup-team-vs-workers-pick-
// canonical) — two adjacent entries for one roster surface read as
// two features. The `/workers` route still resolves; only the
// sidebar entry was dropped.

describe("sidebar nav order", () => {
  it("matches the operator-canonical sequence", () => {
    expect(NAV_ITEMS.map((item) => item.label)).toEqual([
      "Approvals",
      "Team",
      "Genesis",
      "Pitch",
      "Cohorts",
      "Skills",
      "Workflows",
      "Audit",
    ]);
  });

  it("exposes exactly one roster entry (no duplicate Team/Workers pair)", () => {
    const labels = NAV_ITEMS.map((item) => item.label);
    expect(labels).toContain("Team");
    expect(labels).not.toContain("Workers");
  });

  it("places Cohorts before Skills, Workflows and Audit", () => {
    const labels = NAV_ITEMS.map((item) => item.label);
    const cohorts = labels.indexOf("Cohorts");
    expect(cohorts).toBeGreaterThan(-1);
    expect(cohorts).toBeLessThan(labels.indexOf("Skills"));
    expect(cohorts).toBeLessThan(labels.indexOf("Workflows"));
    expect(cohorts).toBeLessThan(labels.indexOf("Audit"));
  });

  it("exposes Genesis immediately after Team", () => {
    const labels = NAV_ITEMS.map((item) => item.label);
    const genesis = labels.indexOf("Genesis");
    expect(genesis).toBeGreaterThan(-1);
    expect(genesis).toBe(labels.indexOf("Team") + 1);
    expect(genesis).toBeLessThan(labels.indexOf("Cohorts"));
  });
});
