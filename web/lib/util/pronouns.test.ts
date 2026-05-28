import { describe, expect, it } from "vitest";
import { formatPronouns } from "./pronouns";
import { SAMPLE_PERSONAS, SAMPLE_PERSONA_IDS, getSamplePersona } from "@/lib/sample-personas";
import { findSampleWorkerOverview } from "@/lib/sample-worker-overview";

/**
 * Guards for fix-worker-pronouns-policy-they-or-explicit-pronoun-field.
 *
 * The brief requires no whitespace-bounded "she" / "he" / "her" /
 * "his" / "him" in a charter unless the persona has explicitly opted
 * into those pronouns. The default for every canonical persona is
 * "they/them" so the inclusive default is the visible one.
 */

const GENDERED = /\b(she|he|her|his|him)\b/i;

describe("formatPronouns helper", () => {
  it("returns the they/them set by default", () => {
    const p = formatPronouns(undefined);
    expect(p.subject).toBe("they");
    expect(p.subjectCap).toBe("They");
    expect(p.object).toBe("them");
    expect(p.possessive).toBe("their");
    expect(p.isPlural).toBe(true);
  });

  it("falls through to they/them on unknown values", () => {
    const p = formatPronouns("xe/xem");
    expect(p.subject).toBe("they");
  });

  it("returns she/her when requested", () => {
    const p = formatPronouns("she/her");
    expect(p.subject).toBe("she");
    expect(p.object).toBe("her");
    expect(p.possessive).toBe("her");
    expect(p.subjectCap).toBe("She");
    expect(p.isPlural).toBe(false);
  });

  it("returns he/him when requested", () => {
    const p = formatPronouns("he/him");
    expect(p.subject).toBe("he");
    expect(p.object).toBe("him");
    expect(p.possessive).toBe("his");
    expect(p.subjectCap).toBe("He");
    expect(p.isPlural).toBe(false);
  });
});

describe("sample-persona pronouns field — operator-settable default", () => {
  it("every canonical persona carries an explicit pronouns string", () => {
    for (const id of SAMPLE_PERSONA_IDS) {
      const persona = getSamplePersona(id);
      expect(persona.pronouns, `${persona.name} is missing a pronouns field`).toBeTruthy();
    }
  });

  it("default for every canonical persona is they/them (inclusive default)", () => {
    for (const persona of Object.values(SAMPLE_PERSONAS)) {
      expect(persona.pronouns, `${persona.name} should default to they/them`).toBe(
        "they/them",
      );
    }
  });

  it("no canonical worker overview charter contains gendered pronouns under the default", () => {
    for (const id of SAMPLE_PERSONA_IDS) {
      const overview = findSampleWorkerOverview(id);
      expect(overview, `missing overview for ${id}`).toBeDefined();
      const persona = getSamplePersona(id);
      if (persona.pronouns !== "they/them") continue;
      expect(
        overview!.charter.match(GENDERED),
        `${persona.name}'s charter still contains gendered pronouns: "${overview!.charter}"`,
      ).toBeNull();
    }
  });
});
