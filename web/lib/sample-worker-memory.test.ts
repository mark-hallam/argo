import { describe, expect, it } from "vitest";
import { findSampleWorkerMemory } from "./sample-worker-memory";

/**
 * Pins Mariah's Memory-tab facts to her marketplace listing role.
 * Scene 1 of the v1.0 demo lingers on /workers/<mariah>/memory while
 * the operator narrates "she remembers the photo standard, she
 * remembers the floor-price rule"; if these facts drift back to
 * procurement-shaped strings the camera reads the wrong policy
 * vocabulary on a marketplace worker.
 */

const MARIAH_ID = "018f67d0-0000-7000-8000-000000000001";

const ROLE_ALIGNED_TERMS = [
  "listing",
  "price",
  "photo",
  "scrape",
  "Ops",
] as const;

const FORBIDDEN_PROCUREMENT_TERMS = [
  "Vendor onboarding",
  "Procurement",
  "strategic partners",
  "Tier 1",
] as const;

describe("sample-worker-memory — Mariah marketplace-listing role alignment", () => {
  it("exposes 5 facts for Mariah", () => {
    const memory = findSampleWorkerMemory(MARIAH_ID);
    expect(memory, "Mariah memory fixture should exist").toBeDefined();
    expect(memory!.facts).toHaveLength(5);
  });

  it("has at least 4 facts that mention listing/price/photo/scrape/Ops", () => {
    const memory = findSampleWorkerMemory(MARIAH_ID)!;
    const alignedCount = memory.facts.filter((fact) => {
      const lower = fact.text.toLowerCase();
      return ROLE_ALIGNED_TERMS.some((term) =>
        lower.includes(term.toLowerCase()),
      );
    }).length;
    expect(
      alignedCount,
      `expected at least 4 of 5 Mariah facts to mention one of [${ROLE_ALIGNED_TERMS.join(", ")}], got ${alignedCount}`,
    ).toBeGreaterThanOrEqual(4);
  });

  it("has zero facts that mention procurement vocabulary", () => {
    const memory = findSampleWorkerMemory(MARIAH_ID)!;
    for (const fact of memory.facts) {
      const lower = fact.text.toLowerCase();
      for (const term of FORBIDDEN_PROCUREMENT_TERMS) {
        expect(
          lower.includes(term.toLowerCase()),
          `Mariah fact "${fact.text}" should not mention "${term}" — that vocabulary belongs to a procurement persona, not a marketplace listing worker`,
        ).toBe(false);
      }
    }
  });
});
