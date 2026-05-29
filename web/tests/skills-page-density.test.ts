/**
 * Skills marketplace density + usage-population pins.
 *
 * Guards the F4/F6 audit fix: the `/skills` hero must not read a sparse
 * "7 skills" on the mock-offline path, and the skill-detail "Workers
 * using this skill" card must never render empty. Both are driven by the
 * static seed catalogue + `seedUsage()` in `lib/api/skills.ts`, so they
 * are unit-testable without a gateway.
 */
import { describe, expect, it } from "vitest";
import { SEED_SKILLS, seedUsage } from "@/lib/api/skills";

// The shared demo roster `seedUsage()` draws from for non-learned skills.
const ROSTER_NAMES: ReadonlySet<string> = new Set([
  "Mariah",
  "Priya",
  "Jamal",
  "Nori",
  "Peta",
  "Patrick",
  "Devon",
  "Alex",
]);

describe("skills marketplace density", () => {
  it("seeds at least 20 skills so the hero count is not sparse", () => {
    expect(SEED_SKILLS.length).toBeGreaterThanOrEqual(20);
  });

  it("spans every business domain the marketplace advertises", () => {
    const names = new Set(SEED_SKILLS.map((s) => s.name));
    // One representative skill per advertised domain.
    const perDomainSample = [
      "invoice.extract", // finance
      "deal.qualify", // sales
      "support.triage", // support
      "standup.summary", // project ops
      "content.repurpose", // marketing
      "candidate.screen", // HR
      "approval.route", // ops
    ];
    for (const name of perDomainSample) {
      expect(names.has(name), `missing domain sample "${name}"`).toBe(true);
    }
  });

  it("gives every seed skill a non-empty usage card", () => {
    for (const skill of SEED_SKILLS) {
      const usage = seedUsage(skill.name);
      expect(
        usage.workers_using_count,
        `"${skill.name}" usage card is empty`,
      ).toBeGreaterThanOrEqual(1);
      expect(usage.workers_using.length).toBe(usage.workers_using_count);
    }
  });
});

describe("seedUsage() worker population", () => {
  it("returns 2-4 distinct roster workers for a non-learned skill", () => {
    const usage = seedUsage("invoice.extract");
    expect(usage.workers_using.length).toBeGreaterThanOrEqual(2);
    expect(usage.workers_using.length).toBeLessThanOrEqual(4);

    const ids = usage.workers_using.map((w) => w.worker_id);
    expect(new Set(ids).size).toBe(ids.length); // distinct

    for (const w of usage.workers_using) {
      expect(ROSTER_NAMES.has(w.name), `non-roster worker "${w.name}"`).toBe(
        true,
      );
      expect(Number.isNaN(Date.parse(w.learned_at))).toBe(false);
    }
  });

  it("is deterministic — same skill always yields the same usage", () => {
    expect(seedUsage("deal.qualify")).toEqual(seedUsage("deal.qualify"));
    expect(seedUsage("vendor.review")).toEqual(seedUsage("vendor.review"));
  });

  it("keeps every adoption count in a plausible 40-300 range", () => {
    for (const skill of SEED_SKILLS) {
      const { adoption_count } = seedUsage(skill.name);
      expect(adoption_count).toBeGreaterThanOrEqual(40);
      expect(adoption_count).toBeLessThanOrEqual(300);
    }
  });

  it("preserves the single learned-by worker for learned skills", () => {
    const usage = seedUsage("support.triage");
    expect(usage.workers_using).toEqual([
      {
        worker_id: "wkr-mariah",
        name: "Mariah",
        learned_at: "2026-05-23T09:15:00.000Z",
      },
    ]);
  });
});
