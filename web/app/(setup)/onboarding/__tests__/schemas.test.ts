/**
 * Unit pins for the onboarding funnel validation schemas (FIX 1).
 *
 * Before this, the first-run inputs gated only on `.trim().length > 0`, so a
 * 5000-char paste, control characters, or a zero-width-only string flowed
 * straight to the gateway and became the worker / workspace display name. The
 * `parseField` + Zod schemas here are the load-bearing stop; the inputs'
 * `maxLength` attribute is the cheap client-side companion.
 */

import { describe, expect, it } from "vitest";
import {
  ONBOARDING_MAX_LENGTHS,
  companyNameSchema,
  firstHireDescriptionSchema,
  firstHireNameSchema,
  parseField,
  providerKeySchema,
  workspaceNameSchema,
} from "../schemas";

const ZERO_WIDTH_SPACE = "​";
const BOM = "﻿";
const WORD_JOINER = "⁠";

describe("workspaceNameSchema", () => {
  it("accepts a normal name and returns the trimmed value", () => {
    const r = parseField(workspaceNameSchema, "  Acme Growth  ");
    expect(r).toEqual({ ok: true, value: "Acme Growth" });
  });

  it("rejects whitespace-only", () => {
    const r = parseField(workspaceNameSchema, "     ");
    expect(r.ok).toBe(false);
  });

  it("rejects a zero-width-only string (survives trim, must not pass)", () => {
    const r = parseField(workspaceNameSchema, ZERO_WIDTH_SPACE);
    expect(r.ok).toBe(false);
  });

  it("rejects a name with an EMBEDDED invisible char (word-joiner / BOM)", () => {
    // Embedded zero-width / word-joiner / BOM survive `.trim()` (only leading
    // & trailing whitespace — and U+FEFF counts as whitespace — are stripped),
    // so they must be caught by the refine.
    expect(parseField(workspaceNameSchema, `Ac${WORD_JOINER}me`).ok).toBe(false);
    expect(parseField(workspaceNameSchema, `Ac${BOM}me`).ok).toBe(false);
  });

  it("accepts a name with only a TRAILING BOM (trim removes it cleanly)", () => {
    // A trailing U+FEFF is whitespace per the spec, so `.trim()` strips it and
    // the cleaned name is valid — no false rejection of a stray paste artifact.
    const r = parseField(workspaceNameSchema, `Acme${BOM}`);
    expect(r).toEqual({ ok: true, value: "Acme" });
  });

  it("rejects a name with a control char (newline pasted into a name)", () => {
    const r = parseField(workspaceNameSchema, "Acme\nGrowth");
    expect(r.ok).toBe(false);
  });

  it("rejects over the 80-char bound", () => {
    const r = parseField(workspaceNameSchema, "a".repeat(81));
    expect(r.ok).toBe(false);
  });

  it("accepts exactly the 80-char bound", () => {
    const r = parseField(workspaceNameSchema, "a".repeat(80));
    expect(r.ok).toBe(true);
  });
});

describe("companyNameSchema", () => {
  it("accepts up to 120 chars", () => {
    expect(parseField(companyNameSchema, "a".repeat(120)).ok).toBe(true);
    expect(parseField(companyNameSchema, "a".repeat(121)).ok).toBe(false);
  });

  it("rejects whitespace-only and zero-width-only", () => {
    expect(parseField(companyNameSchema, "   ").ok).toBe(false);
    expect(parseField(companyNameSchema, ZERO_WIDTH_SPACE).ok).toBe(false);
  });
});

describe("firstHireNameSchema", () => {
  it("accepts a normal hire name", () => {
    expect(parseField(firstHireNameSchema, "Maya").ok).toBe(true);
  });

  it("rejects whitespace-only, zero-width-only, and over-long", () => {
    expect(parseField(firstHireNameSchema, "   ").ok).toBe(false);
    expect(parseField(firstHireNameSchema, ZERO_WIDTH_SPACE).ok).toBe(false);
    expect(parseField(firstHireNameSchema, "n".repeat(81)).ok).toBe(false);
  });
});

describe("firstHireDescriptionSchema", () => {
  it("accepts a multi-line description (newlines are allowed here)", () => {
    const r = parseField(
      firstHireDescriptionSchema,
      "Drafts blog posts.\nRuns weekly competitive scans.",
    );
    expect(r.ok).toBe(true);
  });

  it("rejects whitespace-only and zero-width-only", () => {
    expect(parseField(firstHireDescriptionSchema, "   ").ok).toBe(false);
    expect(parseField(firstHireDescriptionSchema, ZERO_WIDTH_SPACE).ok).toBe(false);
  });

  it("rejects over the 2000-char bound", () => {
    expect(parseField(firstHireDescriptionSchema, "x".repeat(2000)).ok).toBe(true);
    expect(parseField(firstHireDescriptionSchema, "x".repeat(2001)).ok).toBe(false);
  });
});

describe("providerKeySchema", () => {
  it("accepts a well-shaped key", () => {
    expect(parseField(providerKeySchema, "sk-ant-abc123").ok).toBe(true);
  });

  it("rejects a key without the sk-ant- prefix", () => {
    const r = parseField(providerKeySchema, "abc123");
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.error).toMatch(/sk-ant-/);
  });

  it("rejects a runaway over-long paste", () => {
    expect(parseField(providerKeySchema, `sk-ant-${"x".repeat(300)}`).ok).toBe(false);
  });
});

describe("ONBOARDING_MAX_LENGTHS", () => {
  it("matches the schema bounds (HTML maxLength and Zod .max never drift)", () => {
    expect(ONBOARDING_MAX_LENGTHS.workspaceName).toBe(80);
    expect(ONBOARDING_MAX_LENGTHS.companyName).toBe(120);
    expect(ONBOARDING_MAX_LENGTHS.firstHireName).toBe(80);
    expect(ONBOARDING_MAX_LENGTHS.firstHireDescription).toBe(2000);
    expect(ONBOARDING_MAX_LENGTHS.providerKey).toBe(200);
  });
});
