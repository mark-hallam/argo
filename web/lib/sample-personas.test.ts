import { describe, expect, it } from "vitest";
import { SAMPLE_AUDIT_ROWS } from "./sample-audit";
import {
  SAMPLE_PERSONAS,
  SAMPLE_PERSONA_IDS,
  getSamplePersona,
} from "./sample-personas";
import { SAMPLE_TEAM } from "./sample-team";
import { SAMPLE_WORKER_ACTIVITY } from "./sample-worker-activity";
import { findSampleWorkerOverview } from "./sample-worker-overview";

/**
 * Wave-4 data-integrity guard plus Scene-1 roster canon. The five
 * canonical demo workers (Mariah, Peta, Priya, Jamal, Nori) must
 * agree across /team, /audit, /workers/<id>/* surfaces; the
 * spine is `sample-personas.ts` and every other sample file reads
 * from it.
 */

const ROLE_KEYWORDS: Record<string, string[]> = {
  "018f67d0-0000-7000-8000-000000000002": ["candidate", "interview", "JD", "shortlist"],
  "018f67d0-0000-7000-8000-000000000001": ["listing", "buyer", "re-price", "inventory"],
  "018f67d0-0000-7000-8000-000000000003": ["ticket", "refund", "KB", "support"],
  "018f67d0-0000-7000-8000-000000000004": ["draft", "brand", "newsletter", "post", "voice"],
  "018f67d0-0000-7000-8000-000000000007": ["standup", "action item", "sprint", "status"],
};

const FORBIDDEN_OLD_VERBS: Record<string, string[]> = {
  "018f67d0-0000-7000-8000-000000000007": ["invoice", "INV-2024", "reconciliation"],
  "018f67d0-0000-7000-8000-000000000001": ["RFP", "vendor quote", "renewal review"],
  "018f67d0-0000-7000-8000-000000000004": ["pricing rules", "salesforce", "deal-stage", "commission"],
};

describe("sample-personas as the canonical persona spine", () => {
  it("exposes one entry per id in the canonical id list", () => {
    expect(new Set(SAMPLE_PERSONA_IDS).size).toBe(SAMPLE_PERSONA_IDS.length);
    expect(new Set(Object.keys(SAMPLE_PERSONAS))).toEqual(
      new Set(SAMPLE_PERSONA_IDS),
    );
  });

  it("uses gateway-parseable UUID ids for canonical demo personas", () => {
    for (const id of SAMPLE_PERSONA_IDS) {
      expect(id).toMatch(
        /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/,
      );
    }
  });

  it("matches /team — every Worker row reads role + name from canonical", () => {
    for (const persona of Object.values(SAMPLE_PERSONAS)) {
      const teamRow = SAMPLE_TEAM.find((w) => w.id === persona.id);
      expect(teamRow, `missing /team row for ${persona.id}`).toBeDefined();
      expect(teamRow!.name).toBe(persona.name);
      expect(teamRow!.role.tagline).toBe(persona.role.tagline);
      expect(teamRow!.role.vertical).toBe(persona.role.vertical);
    }
  });

  it("matches /workers/<id>/overview — every overview names the worker correctly", () => {
    for (const persona of Object.values(SAMPLE_PERSONAS)) {
      const overview = findSampleWorkerOverview(persona.id);
      expect(overview, `missing overview for ${persona.id}`).toBeDefined();
      expect(
        overview!.charter,
        `overview charter for ${persona.id} should mention ${persona.name}`,
      ).toContain(persona.name);
    }
  });

  it("matches /audit — at least one row per worker, and verbs read as the canonical role", () => {
    for (const id of Object.keys(ROLE_KEYWORDS)) {
      const persona = getSamplePersona(id);
      const rows = SAMPLE_AUDIT_ROWS.filter((r) => r.worker.id === id);
      expect(rows.length, `no audit rows for ${id}`).toBeGreaterThan(0);
      const verbsBlob = rows.map((r) => r.verb).join(" ").toLowerCase();
      const keywords = ROLE_KEYWORDS[id].map((k) => k.toLowerCase());
      const hit = keywords.some((k) => verbsBlob.includes(k));
      expect(
        hit,
        `audit verbs for ${persona.name} should include at least one of [${keywords.join(", ")}] but were: ${verbsBlob.slice(0, 200)}`,
      ).toBe(true);
    }
  });

  it("matches /audit — worker.name in every row equals canonical name", () => {
    for (const row of SAMPLE_AUDIT_ROWS) {
      const persona = SAMPLE_PERSONAS[row.worker.id];
      if (!persona) continue; // workspace + non-persona rows are out of scope
      expect(row.worker.name).toBe(persona.name);
    }
  });

  it("audit verbs no longer carry the split-persona vocabulary (legacy roles)", () => {
    for (const [id, forbidden] of Object.entries(FORBIDDEN_OLD_VERBS)) {
      const rows = SAMPLE_AUDIT_ROWS.filter((r) => r.worker.id === id);
      const verbsBlob = rows.map((r) => r.verb).join(" ").toLowerCase();
      for (const term of forbidden) {
        expect(
          verbsBlob.includes(term.toLowerCase()),
          `audit verbs for ${id} should no longer mention "${term}" — that was the old split-persona role`,
        ).toBe(false);
      }
    }
  });

  it("matches /workers/<id>/activity — worker name + verbs read as canonical role", () => {
    for (const id of Object.keys(ROLE_KEYWORDS)) {
      const rows = SAMPLE_WORKER_ACTIVITY[id];
      if (!rows) continue; // not every persona has an activity feed
      const persona = getSamplePersona(id);
      for (const row of rows) {
        expect(row.workerName).toBe(persona.name);
      }
      const verbsBlob = rows.map((r) => r.verb).join(" ").toLowerCase();
      const keywords = ROLE_KEYWORDS[id].map((k) => k.toLowerCase());
      const hit = keywords.some((k) => verbsBlob.includes(k));
      expect(
        hit,
        `activity verbs for ${persona.name} should include one of [${keywords.join(", ")}]`,
      ).toBe(true);
    }
  });
});
