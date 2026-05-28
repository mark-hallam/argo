/**
 * Unit tests for the placeholder lookup behind step 5's textarea. The
 * Playwright spec covers the browser-side wiring; this file pins the
 * pure-function contract so a typo in the lookup table fails fast in
 * the vitest tier.
 */

import { describe, expect, it } from "vitest";
import {
  DEFAULT_EXAMPLE_PROMPT,
  EXAMPLE_PROMPTS,
  exampleForVertical,
} from "@/lib/onboarding/example-prompts";

describe("example-prompts lookup", () => {
  it("returns the marketing-intern copy as the default", () => {
    expect(DEFAULT_EXAMPLE_PROMPT).toMatch(/Maya/);
    expect(DEFAULT_EXAMPLE_PROMPT).toMatch(/marketing intern/i);
  });

  it("falls back to the default for null / undefined / unknown ids", () => {
    expect(exampleForVertical(null)).toBe(DEFAULT_EXAMPLE_PROMPT);
    expect(exampleForVertical(undefined)).toBe(DEFAULT_EXAMPLE_PROMPT);
    expect(exampleForVertical("not-a-real-vertical")).toBe(DEFAULT_EXAMPLE_PROMPT);
  });

  it("returns the GTM-engineer placeholder for the gtm-engineer id", () => {
    const example = exampleForVertical("gtm-engineer");
    expect(example).toMatch(/Mariah|GTM engineer/i);
    expect(example).not.toMatch(/marketing intern/i);
  });

  it("returns the recruiter placeholder for the recruiter id", () => {
    const example = exampleForVertical("recruiter");
    expect(example).toMatch(/Rachel|recruiter/i);
    expect(example).not.toMatch(/marketing intern/i);
  });

  it("covers every vertical that pick-vertical can stage", () => {
    // Source of truth: the ids the picker can set on the draft store.
    // If a new vertical card lands in step 4 without a placeholder
    // entry, step 5 silently falls back to marketing-intern — and the
    // brief's whole premise (placeholder matches picked vertical)
    // breaks. Pin the list so adding a card forces an example update.
    const KNOWN_PICKER_IDS = [
      "gtm-engineer",
      "marketing-intern",
      "marketplace-seller",
      "recruiter",
      "sales-sdr",
      "customer-support",
      "finance-ar",
    ];
    for (const id of KNOWN_PICKER_IDS) {
      expect(EXAMPLE_PROMPTS, `missing placeholder for vertical id "${id}"`).toHaveProperty(
        id,
      );
    }
  });

  it("avoids cadence words (weekly/daily) per Phase 1 copy-honesty rule", () => {
    // The wizard only POSTs {description, vertical_id}; cadence is set
    // later on the worker detail page's Schedules card. Promising
    // weekly/daily cadence in the placeholder writes a cheque the
    // wiring can't cash. See
    // `fix-first-hire-cta-copy-promises-weekly-schedule-not-parsed`.
    for (const [id, copy] of Object.entries(EXAMPLE_PROMPTS)) {
      expect(copy, `vertical "${id}" placeholder mentions weekly/daily cadence`).not.toMatch(
        /\bweekly\b|\bdaily\b/i,
      );
    }
  });
});
