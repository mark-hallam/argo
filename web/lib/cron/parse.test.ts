import { describe, expect, it } from "vitest";
import { CRON_PRESETS, parseCron } from "./parse";

describe("parseCron — valid expressions", () => {
  it("accepts all-wildcards", () => {
    expect(parseCron("* * * * *")).toEqual({ ok: true });
  });

  it("accepts a single fixed minute/hour combo", () => {
    expect(parseCron("0 6 * * *")).toEqual({ ok: true });
  });

  it("accepts step expressions in minute and hour fields", () => {
    expect(parseCron("*/15 */2 * * *")).toEqual({ ok: true });
  });

  it("accepts a day-of-week range", () => {
    expect(parseCron("0 9 * * 1-5")).toEqual({ ok: true });
  });

  it("accepts a comma list with mixed atoms", () => {
    expect(parseCron("0,15,30,45 * * * *")).toEqual({ ok: true });
  });

  it("accepts a comma list containing a range", () => {
    expect(parseCron("0 9 * * 1-3,5")).toEqual({ ok: true });
  });

  it("accepts a base-range step", () => {
    expect(parseCron("0 9-17/2 * * *")).toEqual({ ok: true });
  });

  it("tolerates surrounding whitespace", () => {
    expect(parseCron("  0 6 * * *  ")).toEqual({ ok: true });
  });

  it("accepts every documented preset", () => {
    for (const preset of CRON_PRESETS) {
      expect(parseCron(preset.spec)).toEqual({ ok: true });
    }
  });
});

describe("parseCron — invalid expressions", () => {
  it("rejects an empty string", () => {
    const r = parseCron("");
    expect(r.ok).toBe(false);
  });

  it("rejects four fields", () => {
    const r = parseCron("0 6 * *");
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.reason).toMatch(/5 fields/);
  });

  it("rejects six fields", () => {
    const r = parseCron("0 6 * * * *");
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.reason).toMatch(/5 fields/);
  });

  it("rejects out-of-range minute", () => {
    const r = parseCron("60 * * * *");
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.reason).toMatch(/minute/);
  });

  it("rejects out-of-range hour", () => {
    const r = parseCron("0 24 * * *");
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.reason).toMatch(/hour/);
  });

  it("rejects day-of-month 0", () => {
    const r = parseCron("0 0 0 * *");
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.reason).toMatch(/day-of-month/);
  });

  it("rejects month 13", () => {
    const r = parseCron("0 0 1 13 *");
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.reason).toMatch(/month/);
  });

  it("rejects day-of-week 7", () => {
    const r = parseCron("0 0 * * 7");
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.reason).toMatch(/day-of-week/);
  });

  it("rejects reversed range", () => {
    const r = parseCron("0 9 * * 5-1");
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.reason).toMatch(/reversed/);
  });

  it("rejects step zero", () => {
    const r = parseCron("*/0 * * * *");
    expect(r.ok).toBe(false);
  });

  it("rejects non-numeric atoms", () => {
    const r = parseCron("foo * * * *");
    expect(r.ok).toBe(false);
  });

  it("rejects empty list entries", () => {
    const r = parseCron("0,,30 * * * *");
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.reason).toMatch(/Empty value/);
  });

  it("rejects @daily and other special tokens", () => {
    const r = parseCron("@daily");
    expect(r.ok).toBe(false);
  });
});
