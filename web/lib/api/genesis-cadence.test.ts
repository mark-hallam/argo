import { describe, expect, it } from "vitest";
import {
  DEFAULT_CADENCE_CRON,
  parseCadenceToCron,
} from "./genesis-cadence";

describe("parseCadenceToCron", () => {
  it("parses 'every weekday at 5pm' to 17:00 weekdays (the live-repro case)", () => {
    const { cron, parsed } = parseCadenceToCron("every weekday at 5pm");
    expect(cron).toBe("0 17 * * 1-5");
    expect(parsed).toBe(true);
  });

  it("parses a daily time", () => {
    expect(parseCadenceToCron("daily at 9am")).toEqual({
      cron: "0 9 * * *",
      parsed: true,
    });
  });

  it("parses a named weekday with a time", () => {
    expect(parseCadenceToCron("every Monday at 9:30am")).toEqual({
      cron: "30 9 * * 1",
      parsed: true,
    });
  });

  it("parses 24-hour colon times", () => {
    expect(parseCadenceToCron("weekdays at 17:00")).toEqual({
      cron: "0 17 * * 1-5",
      parsed: true,
    });
  });

  it("parses 'hourly' as a whole-field schedule", () => {
    expect(parseCadenceToCron("run hourly")).toEqual({
      cron: "0 * * * *",
      parsed: true,
    });
  });

  it("parses 'weekly' to a Monday default when no day is named", () => {
    expect(parseCadenceToCron("weekly")).toEqual({
      cron: "0 9 * * 1",
      parsed: true,
    });
  });

  it("handles dotted p.m.", () => {
    expect(parseCadenceToCron("every day at 5 p.m.")).toEqual({
      cron: "0 17 * * *",
      parsed: true,
    });
  });

  it("falls back to the default cron and flags unparsed for free text", () => {
    expect(parseCadenceToCron("whenever an inbound ticket arrives")).toEqual({
      cron: DEFAULT_CADENCE_CRON,
      parsed: false,
    });
  });

  it("falls back for empty / missing cadence", () => {
    expect(parseCadenceToCron("")).toEqual({
      cron: DEFAULT_CADENCE_CRON,
      parsed: false,
    });
    expect(parseCadenceToCron(null)).toEqual({
      cron: DEFAULT_CADENCE_CRON,
      parsed: false,
    });
    expect(parseCadenceToCron(undefined)).toEqual({
      cron: DEFAULT_CADENCE_CRON,
      parsed: false,
    });
  });
});
