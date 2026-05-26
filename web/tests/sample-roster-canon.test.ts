/**
 * Scene-1 roster canon test. Every attribution field on the demo
 * sample fixtures (workflow `lastEditor`, approval `worker.name`,
 * etc.) must read as one of the canonical 5 demo workers — Mariah,
 * Peta, Priya, Jamal, Nori. Catches "by Rio"-style camera-visible
 * scene-vocabulary mismatches before they reach the demo MP4.
 */
import { describe, expect, it } from "vitest";
import { SAMPLE_WORKFLOW_SUMMARIES } from "@/lib/sample-workflows";
import { SAMPLE_APPROVALS } from "@/lib/sample-approvals";
import { SAMPLE_PERSONAS, SAMPLE_PERSONA_IDS } from "@/lib/sample-personas";
import { SAMPLE_AUDIT_ROWS } from "@/lib/sample-audit";
import { SAMPLE_AUDIT_DETAILS } from "@/lib/sample-audit-detail";
import { SAMPLE_TEAM } from "@/lib/sample-team";

const CANONICAL_NAMES: ReadonlySet<string> = new Set([
  "Mariah",
  "Peta",
  "Priya",
  "Jamal",
  "Nori",
]);

describe("Scene-1 canonical roster appears on every demo sample surface", () => {
  it("SAMPLE_PERSONA_IDS contains exactly the canonical five", () => {
    const names = SAMPLE_PERSONA_IDS.map((id) => SAMPLE_PERSONAS[id]?.name);
    expect(new Set(names)).toEqual(CANONICAL_NAMES);
    expect(SAMPLE_PERSONA_IDS.length).toBe(5);
  });

  it("workflow card lastEditor reads as a canonical name", () => {
    for (const row of SAMPLE_WORKFLOW_SUMMARIES) {
      expect(
        CANONICAL_NAMES.has(row.lastEditor),
        `workflow ${row.name} has non-canonical editor "${row.lastEditor}"`,
      ).toBe(true);
    }
  });

  it("approvals inbox worker.name reads as a canonical name", () => {
    for (const item of SAMPLE_APPROVALS) {
      expect(
        CANONICAL_NAMES.has(item.worker.name),
        `approval ${item.id} attributed to non-canonical "${item.worker.name}"`,
      ).toBe(true);
    }
  });

  it("audit feed worker.name resolves to a canonical name", () => {
    for (const row of SAMPLE_AUDIT_ROWS) {
      const persona = SAMPLE_PERSONAS[row.worker.id];
      if (!persona) continue;
      expect(
        CANONICAL_NAMES.has(persona.name),
        `audit row ${row.id} attributed to non-canonical "${persona.name}"`,
      ).toBe(true);
    }
  });

  it("hand-crafted audit details name a canonical worker", () => {
    for (const detail of SAMPLE_AUDIT_DETAILS) {
      expect(
        CANONICAL_NAMES.has(detail.worker.name),
        `audit detail ${detail.id} attributed to non-canonical "${detail.worker.name}"`,
      ).toBe(true);
    }
  });

  it("/team roster shows only canonical names", () => {
    for (const worker of SAMPLE_TEAM) {
      expect(
        CANONICAL_NAMES.has(worker.name),
        `/team row ${worker.id} surfaces non-canonical "${worker.name}"`,
      ).toBe(true);
    }
    expect(SAMPLE_TEAM.length).toBe(5);
  });
});
