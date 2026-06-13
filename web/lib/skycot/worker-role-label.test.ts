import { describe, expect, it } from "vitest";
import type { Worker } from "@/lib/types/worker";
import {
  humanizeSlug,
  isGenesisProvenanceMarker,
  resolveRoleLabel,
} from "./worker-role-label";

function makeWorker(overrides: Partial<Worker> = {}): Worker {
  return {
    id: "wkr_test",
    name: "Rex",
    role: { tagline: "", vertical: "collections" },
    status: "working",
    ...overrides,
  };
}

describe("isGenesisProvenanceMarker", () => {
  it("matches the 'Genesis <name> <plan_id>' marker", () => {
    expect(isGenesisProvenanceMarker("Genesis Rex 019ebea4d72d1b81")).toBe(true);
    expect(isGenesisProvenanceMarker("genesis ivy 0a1b2c3d4e5f6071")).toBe(true);
  });

  it("does not match a real human role title", () => {
    expect(isGenesisProvenanceMarker("Senior Recruiter (AI)")).toBe(false);
    expect(isGenesisProvenanceMarker("Collections Coordinator")).toBe(false);
    // "Genesis" as a legit team name without a plan id is not a marker.
    expect(isGenesisProvenanceMarker("Genesis Team Lead")).toBe(false);
  });
});

describe("resolveRoleLabel", () => {
  it("shows a real human tagline as-is", () => {
    expect(
      resolveRoleLabel(makeWorker({ role: { tagline: "Senior Recruiter (AI)", vertical: "recruiter" } })),
    ).toBe("Senior Recruiter (AI)");
  });

  it("never returns the Genesis provenance marker (F13)", () => {
    const label = resolveRoleLabel(
      makeWorker({ role: { tagline: "Genesis Rex 019ebea4d72d1b81", vertical: "collections" } }),
    );
    expect(label).toBe("Collections");
    expect(label).not.toMatch(/Genesis|019ebea4d72d1b81/);
  });

  it("falls back to a humanized vertical when the tagline is a bare name slug", () => {
    expect(
      resolveRoleLabel(makeWorker({ name: "Ivy", role: { tagline: "ivy", vertical: "collections" } })),
    ).toBe("Collections");
  });

  it("humanizes a non-name slug tagline", () => {
    expect(
      resolveRoleLabel(makeWorker({ role: { tagline: "collections-coordinator", vertical: "collections" } })),
    ).toBe("Collections Coordinator");
  });

  it("falls back to 'Teammate' when nothing usable is captured", () => {
    expect(resolveRoleLabel(makeWorker({ role: { tagline: "", vertical: "" } }))).toBe("Teammate");
    // Provenance marker + no vertical → still never the marker.
    expect(
      resolveRoleLabel(makeWorker({ role: { tagline: "Genesis Rex 019ebea4d72d1b81", vertical: "" } })),
    ).toBe("Teammate");
  });
});

describe("humanizeSlug", () => {
  it("title-cases a hyphen/underscore slug", () => {
    expect(humanizeSlug("collections-coordinator")).toBe("Collections Coordinator");
    expect(humanizeSlug("sales_dev")).toBe("Sales Dev");
  });
});
