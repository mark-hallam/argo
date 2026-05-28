/**
 * Demo-mode short-circuit + vertical-to-sample-worker fallback for
 * the first-hire wizard. Pins:
 *   - every vertical in the wizard's catalogue maps to a defined
 *     SAMPLE_* UUID present in the sample-personas spine
 *   - `project-coordination` routes to Peta (closes F14)
 *   - unknown verticals fall back to Mariah (Scene-3 demo runbook)
 */

import { describe, expect, it, beforeEach, vi } from "vitest";
import {
  SAMPLE_JAMAL_ID,
  SAMPLE_MARIAH_ID,
  SAMPLE_NORI_ID,
  SAMPLE_PETA_ID,
  SAMPLE_PRIYA_ID,
  SAMPLE_PERSONAS,
} from "@/lib/sample-personas";

vi.mock("@/lib/skycot/demo-mode", () => ({
  isDemoMode: () => true,
}));

import {
  DEMO_SUPPORTED_VERTICAL_IDS,
  firstHire,
} from "@/lib/onboarding/use-first-hire";

const EXPECTED_VERTICALS: Record<string, { id: string; name: string }> = {
  "marketplace-seller": { id: SAMPLE_MARIAH_ID, name: "Mariah" },
  recruiter: { id: SAMPLE_PRIYA_ID, name: "Priya" },
  "customer-support": { id: SAMPLE_JAMAL_ID, name: "Jamal" },
  "marketing-intern": { id: SAMPLE_NORI_ID, name: "Nori" },
  "project-coordination": { id: SAMPLE_PETA_ID, name: "Peta" },
};

describe("firstHire (demo mode)", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("routes project-coordination to Peta (F14)", async () => {
    const result = await firstHire({
      name: "Peta",
      description: "Coordinate the standups",
      verticalId: "project-coordination",
    });
    expect(result.workerId).toBe(SAMPLE_PETA_ID);
    expect(result.workerName).toBe("Peta");
    expect(result.pending).toBe(false);
  });

  it("every mapped vertical resolves to a UUID present in the sample-personas spine", async () => {
    for (const [verticalId, expected] of Object.entries(EXPECTED_VERTICALS)) {
      const result = await firstHire({
        name: expected.name,
        description: `Hire for ${verticalId}`,
        verticalId,
      });
      expect(result.workerId, `vertical ${verticalId} routes to wrong id`).toBe(expected.id);
      expect(result.workerName, `vertical ${verticalId} routes to wrong name`).toBe(
        expected.name,
      );
      expect(
        SAMPLE_PERSONAS[result.workerId],
        `vertical ${verticalId} returned UUID ${result.workerId} not in SAMPLE_PERSONAS`,
      ).toBeDefined();
    }
  });

  it("unknown verticals fall back to Mariah (Scene-3 demo runbook)", async () => {
    const result = await firstHire({
      name: "anyone",
      description: "Whatever",
      verticalId: "completely-bogus-vertical",
    });
    expect(result.workerId).toBe(SAMPLE_MARIAH_ID);
    expect(result.workerName).toBe("Mariah");
  });

  it("Peta is exported from sample-personas and is reachable (closes F14 SAMPLE_PETA-missing gap)", () => {
    expect(SAMPLE_PETA_ID).toBeTruthy();
    expect(SAMPLE_PERSONAS[SAMPLE_PETA_ID]).toBeDefined();
    expect(SAMPLE_PERSONAS[SAMPLE_PETA_ID].name).toBe("Peta");
  });

  it("F10 — every DEMO_SUPPORTED vertical resolves to a non-Mariah-fallback persona (except marketplace-seller)", async () => {
    // The set the pick-vertical grid filters against must, by
    // construction, contain ONLY verticals whose `firstHire` result is
    // the mapped persona — never the silent fall-through to
    // DEMO_FALLBACK_WORKER (Mariah). Marketplace-seller legitimately
    // maps to Mariah; every other supported id must return a different
    // persona so we can distinguish "mapped to Mariah" from
    // "fell-through to Mariah".
    expect(DEMO_SUPPORTED_VERTICAL_IDS.size).toBeGreaterThanOrEqual(4);
    for (const verticalId of DEMO_SUPPORTED_VERTICAL_IDS) {
      const result = await firstHire({
        name: "anyone",
        description: `Hire for ${verticalId}`,
        verticalId,
      });
      expect(SAMPLE_PERSONAS[result.workerId]).toBeDefined();
      if (verticalId !== "marketplace-seller") {
        expect(
          result.workerId,
          `vertical ${verticalId} silently fell through to Mariah`,
        ).not.toBe(SAMPLE_MARIAH_ID);
      }
    }
  });
});
