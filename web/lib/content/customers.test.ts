import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

import { SEED_CUSTOMERS, type CustomerStory } from "./customers";

// Copy-honesty pins for web-customers-page-descriptive-scenarios-not-fake-quotes
// (shakedown F3). /customers must read as third-person SCENARIOS, never as
// fabricated testimonials. Two load-bearing rules:
//   1. No quote-attribution pattern ("— Name, Title at Company") anywhere the
//      page renders — and no author field carrying such attribution.
//   2. The word HIPAA appears only with explicit posture framing (in SKYCOT's
//      own voice), or not at all. AFSL claims are held to the same standard.
// /security is the honesty benchmark; this brings /customers up to it.

const THIS_DIR = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = join(THIS_DIR, "..", "..", "..");
const CONTENT_SRC = join(THIS_DIR, "customers.ts");
const INDEX_PAGE = join(REPO_ROOT, "web", "app", "customers", "page.tsx");
const DETAIL_PAGE = join(REPO_ROOT, "web", "app", "customers", "[slug]", "page.tsx");

/** Every customer-visible string value on a scenario, flattened. */
function visibleStrings(story: CustomerStory): string[] {
  const out: string[] = [
    story.name,
    story.industry,
    story.size,
    story.vertical,
    story.cardKpi,
    story.headlineKpi,
    story.cardQuote,
    story.heroPullQuote,
    story.secondQuote,
    ...story.setupParagraphs,
    ...story.metrics.flatMap((m) => [m.value, m.label]),
  ];
  if (story.compliancePosture) out.push(story.compliancePosture);
  return out;
}

// "— Priya Subramaniam, VP Support at Northwind" and the role-at-company
// testimonial signature it implies. Both forms must be absent from rendered copy.
const EMDASH_ATTRIBUTION = /[—–-]\s*[A-Z][a-zA-Z.]+\s+[A-Z][a-zA-Z]+,/;
const ROLE_AT_COMPANY =
  /,\s+(?:VP|Head|Chief|Director|Founder|CEO|MD|Managing Director|Senior|Coordinator|Officer|Analyst)\b[^."]*\bat\s+[A-Z]/;

describe("/customers scenarios carry no fabricated person-quotes", () => {
  for (const story of SEED_CUSTOMERS) {
    it(`${story.slug} has no quote-attribution in any visible string`, () => {
      for (const s of visibleStrings(story)) {
        expect(
          EMDASH_ATTRIBUTION.test(s),
          `"${story.slug}" string reads as an attributed quote ("— Name, …"): ${s}`,
        ).toBe(false);
        expect(
          ROLE_AT_COMPANY.test(s),
          `"${story.slug}" string reads as a "<Role> at <Company>" testimonial: ${s}`,
        ).toBe(false);
      }
    });
  }

  it("the content model carries no author/attribution fields", () => {
    const src = readFileSync(CONTENT_SRC, "utf8");
    expect(
      /QuoteAuthor/.test(src),
      "customers.ts still declares a *QuoteAuthor field — attribution must be removed entirely.",
    ).toBe(false);
    // The runtime objects must not carry an attribution property either.
    for (const story of SEED_CUSTOMERS) {
      for (const key of Object.keys(story)) {
        expect(
          /author/i.test(key),
          `${story.slug} carries an attribution-shaped field "${key}".`,
        ).toBe(false);
      }
    }
  });

  it("neither /customers page renders a quote-author element", () => {
    for (const page of [INDEX_PAGE, DETAIL_PAGE]) {
      const src = readFileSync(page, "utf8");
      expect(
        /QuoteAuthor/.test(src),
        `${page} still renders a *QuoteAuthor attribution.`,
      ).toBe(false);
    }
  });
});

describe("/customers states compliance only in SKYCOT's own voice", () => {
  it("every HIPAA mention carries explicit posture framing", () => {
    const src = readFileSync(CONTENT_SRC, "utf8");
    for (const rawLine of src.split("\n")) {
      if (!/HIPAA/.test(rawLine)) continue;
      // JSDoc lines (comments) describe the rule and are allowed.
      const isComment = /^\s*(\*|\/\/|\/\*)/.test(rawLine);
      if (isComment) continue;
      expect(
        /posture/i.test(rawLine),
        `HIPAA mentioned without explicit posture framing on the same line: ${rawLine.trim()}`,
      ).toBe(true);
    }
  });

  it("any HIPAA/AFSL framework mention lives only in compliancePosture", () => {
    for (const story of SEED_CUSTOMERS) {
      const nonPosture = visibleStrings(story).filter((s) => s !== story.compliancePosture);
      for (const s of nonPosture) {
        // AFSL as licence context ("AFSL holder") is factual and allowed; a
        // "AFSL-compliant" style assertion is not. Guard the assertion form.
        expect(
          /\bHIPAA\b/.test(s),
          `${story.slug} names HIPAA outside compliancePosture: ${s}`,
        ).toBe(false);
        expect(
          /\bAFSL[\s-]?compliant/i.test(s),
          `${story.slug} asserts AFSL compliance outside SKYCOT's own voice: ${s}`,
        ).toBe(false);
      }
    }
  });
});
