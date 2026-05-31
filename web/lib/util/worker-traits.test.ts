import { describe, expect, it } from "vitest";
import {
  parseTraitsFromTagline,
  stripTraitsSuffix,
} from "./worker-traits";

/**
 * The fixtures below are copied verbatim from the live gateway's
 * `GET /api/v1/team` response (the demo-seed format in
 * `crates/argo-demo-seed/src/seed.rs`). Keeping real strings here is
 * the contract guard: if the seed ever changes its tagline shape this
 * test breaks loudly rather than the personality card silently falling
 * back to generic boilerplate again.
 */
const MARCUS_TAGLINE =
  "Marketing: drafts replies + thought-leadership in Mark's voice — draft-only, never auto-posted. Traits: On-voice, Listening-first, Truth-bounded, Draft-only.";
const MARIAH_TAGLINE =
  "Marketplace Lister: lists and re-prices items for sale. Traits: Patient, Detail-oriented, Picture-perfect listings, Polite.";

describe("parseTraitsFromTagline", () => {
  it("pulls the real traits out of a seeded gateway tagline", () => {
    expect(parseTraitsFromTagline(MARCUS_TAGLINE)).toEqual([
      "On-voice",
      "Listening-first",
      "Truth-bounded",
      "Draft-only",
    ]);
  });

  it("handles multi-word traits with internal spaces + hyphens", () => {
    expect(parseTraitsFromTagline(MARIAH_TAGLINE)).toEqual([
      "Patient",
      "Detail-oriented",
      "Picture-perfect listings",
      "Polite",
    ]);
  });

  it("returns [] when the tagline has no Traits: suffix", () => {
    expect(parseTraitsFromTagline("books showings and answers chats")).toEqual(
      [],
    );
  });

  it("returns [] for empty / nullish taglines", () => {
    expect(parseTraitsFromTagline("")).toEqual([]);
    expect(parseTraitsFromTagline(undefined)).toEqual([]);
    expect(parseTraitsFromTagline(null)).toEqual([]);
  });

  it("does not choke when the description itself mentions 'Traits'", () => {
    // lastIndexOf anchors on the real suffix, not an in-body mention.
    const tagline =
      "Coaches teammates on their Traits. Traits: Patient, Direct.";
    expect(parseTraitsFromTagline(tagline)).toEqual(["Patient", "Direct"]);
  });

  it("tolerates a missing trailing period", () => {
    expect(parseTraitsFromTagline("Role: x. Traits: A, B")).toEqual(["A", "B"]);
  });
});

describe("stripTraitsSuffix", () => {
  it("removes the trailing Traits: clause and keeps the role line", () => {
    expect(stripTraitsSuffix(MARIAH_TAGLINE)).toBe(
      "Marketplace Lister: lists and re-prices items for sale.",
    );
  });

  it("leaves a tagline without a suffix unchanged", () => {
    expect(stripTraitsSuffix("books showings")).toBe("books showings");
  });

  it("returns '' for nullish input", () => {
    expect(stripTraitsSuffix(undefined)).toBe("");
    expect(stripTraitsSuffix(null)).toBe("");
  });
});
