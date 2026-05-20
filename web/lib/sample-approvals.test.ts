import { describe, expect, it } from "vitest";
import { sortBySeverityThenSla, type ApprovalItem } from "./sample-approvals";

const NOW = new Date("2026-05-21T12:00:00Z").getTime();

function item(over: Partial<ApprovalItem>): ApprovalItem {
  return {
    id: over.id ?? "x",
    worker: { name: "Worker" },
    verb: "wants to",
    detail: "detail",
    consequenceIfYes: "yes",
    consequenceIfNo: "no",
    channel: "slack",
    receivedAt: new Date(NOW - 60_000).toISOString(),
    ...over,
  };
}

describe("sortBySeverityThenSla", () => {
  it("places P0 items before P1 before P2", () => {
    const out = sortBySeverityThenSla([
      item({ id: "p2", severity: "p2" }),
      item({ id: "p0", severity: "p0" }),
      item({ id: "p1", severity: "p1" }),
    ]);
    expect(out.map((i) => i.id)).toEqual(["p0", "p1", "p2"]);
  });

  it("breaks severity ties by SLA ascending (earliest deadline first)", () => {
    const out = sortBySeverityThenSla([
      item({
        id: "later",
        severity: "p0",
        slaExpiresAt: new Date(NOW + 4 * 60 * 60 * 1000).toISOString(),
      }),
      item({
        id: "sooner",
        severity: "p0",
        slaExpiresAt: new Date(NOW + 1 * 60 * 60 * 1000).toISOString(),
      }),
    ]);
    expect(out.map((i) => i.id)).toEqual(["sooner", "later"]);
  });

  it("treats items without severity as P2", () => {
    const out = sortBySeverityThenSla([
      item({ id: "p0", severity: "p0" }),
      item({ id: "default" }),
    ]);
    expect(out.map((i) => i.id)).toEqual(["p0", "default"]);
  });

  it("does not mutate the input list", () => {
    const input = [
      item({ id: "p2", severity: "p2" }),
      item({ id: "p0", severity: "p0" }),
    ];
    const snapshot = input.map((i) => i.id);
    sortBySeverityThenSla(input);
    expect(input.map((i) => i.id)).toEqual(snapshot);
  });
});
