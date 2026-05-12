import { describe, expect, it } from "vitest";
import type { ApprovalItem } from "@/lib/sample-approvals";
import {
  EMPTY_FILTERS,
  activeFilterCount,
  applyFilters,
  clearDimension,
  isFilterEmpty,
  parseFilters,
  serializeFilters,
  toggleFilterValue,
  workerOptions,
} from "./approvals";

function makeItem(overrides: Partial<ApprovalItem> = {}): ApprovalItem {
  return {
    id: "ap_test",
    worker: { name: "Mariah" },
    verb: "wants to send a thing",
    detail: "Some detail",
    consequenceIfYes: "yes",
    consequenceIfNo: "no",
    channel: "slack",
    receivedAt: "2026-05-12T10:00:00Z",
    decision: null,
    ...overrides,
  };
}

const sampleItems: ApprovalItem[] = [
  makeItem({ id: "1", worker: { name: "Mariah" }, channel: "slack", decision: null }),
  makeItem({ id: "2", worker: { name: "Priya" }, channel: "imessage", decision: null }),
  makeItem({ id: "3", worker: { name: "Jamal" }, channel: "web", decision: "approve" }),
  makeItem({ id: "4", worker: { name: "Mariah" }, channel: "push", decision: "refuse" }),
  makeItem({ id: "5", worker: { name: "Nori" }, channel: "slack", decision: null }),
];

describe("approvals filters", () => {
  it("empty filter passes everything through", () => {
    expect(isFilterEmpty(EMPTY_FILTERS)).toBe(true);
    expect(applyFilters(sampleItems, EMPTY_FILTERS)).toEqual(sampleItems);
  });

  it("single-dimension worker filter selects only matching items", () => {
    const filters = { ...EMPTY_FILTERS, worker: ["Mariah"] };
    const result = applyFilters(sampleItems, filters);
    expect(result.map((i) => i.id)).toEqual(["1", "4"]);
  });

  it("single-dimension channel filter ORs within the dimension", () => {
    const filters = { ...EMPTY_FILTERS, channel: ["slack" as const, "push" as const] };
    const result = applyFilters(sampleItems, filters);
    expect(result.map((i) => i.id)).toEqual(["1", "4", "5"]);
  });

  it("status filter recognizes pending/approved/refused based on decision field", () => {
    expect(
      applyFilters(sampleItems, { ...EMPTY_FILTERS, status: ["pending"] }).map((i) => i.id),
    ).toEqual(["1", "2", "5"]);
    expect(
      applyFilters(sampleItems, { ...EMPTY_FILTERS, status: ["approved"] }).map((i) => i.id),
    ).toEqual(["3"]);
    expect(
      applyFilters(sampleItems, { ...EMPTY_FILTERS, status: ["refused"] }).map((i) => i.id),
    ).toEqual(["4"]);
  });

  it("multi-dimension filter ANDs across dimensions", () => {
    const filters = {
      worker: ["Mariah"],
      channel: ["slack" as const],
      status: ["pending" as const],
    };
    expect(applyFilters(sampleItems, filters).map((i) => i.id)).toEqual(["1"]);
  });

  it("multi-dimension filter that excludes all returns empty", () => {
    const filters = {
      worker: ["Priya"],
      channel: ["slack" as const],
      status: ["pending" as const],
    };
    expect(applyFilters(sampleItems, filters)).toEqual([]);
  });

  it("URL round-trip preserves filter shape", () => {
    const filters = {
      worker: ["Priya", "Mariah"],
      channel: ["slack" as const, "push" as const],
      status: ["pending" as const, "approved" as const],
    };
    const serialized = serializeFilters(filters);
    const parsed = parseFilters(new URLSearchParams(serialized));
    expect(parsed.worker).toEqual(["Mariah", "Priya"]);
    expect(parsed.channel).toEqual(["push", "slack"]);
    expect(parsed.status).toEqual(["approved", "pending"]);
    expect(serialized).toBe(
      "worker=Mariah%2CPriya&channel=push%2Cslack&status=approved%2Cpending",
    );
  });

  it("parse drops unknown channel/status tokens silently", () => {
    const parsed = parseFilters(
      new URLSearchParams("channel=slack,bogus&status=pending,gibberish"),
    );
    expect(parsed.channel).toEqual(["slack"]);
    expect(parsed.status).toEqual(["pending"]);
  });

  it("toggle adds and removes values within a dimension", () => {
    let f = EMPTY_FILTERS;
    f = toggleFilterValue(f, "worker", "Mariah");
    expect(f.worker).toEqual(["Mariah"]);
    f = toggleFilterValue(f, "worker", "Priya");
    expect(f.worker).toEqual(["Mariah", "Priya"]);
    f = toggleFilterValue(f, "worker", "Mariah");
    expect(f.worker).toEqual(["Priya"]);
  });

  it("clearDimension removes only the targeted dimension", () => {
    const f = {
      worker: ["Mariah"],
      channel: ["slack" as const],
      status: ["pending" as const],
    };
    const cleared = clearDimension(f, "channel");
    expect(cleared).toEqual({
      worker: ["Mariah"],
      channel: [],
      status: ["pending"],
    });
  });

  it("workerOptions returns alphabetically-sorted unique names", () => {
    expect(workerOptions(sampleItems)).toEqual(["Jamal", "Mariah", "Nori", "Priya"]);
  });

  it("activeFilterCount counts dimensions with at least one value", () => {
    expect(activeFilterCount(EMPTY_FILTERS)).toBe(0);
    expect(activeFilterCount({ ...EMPTY_FILTERS, worker: ["Mariah"] })).toBe(1);
    expect(
      activeFilterCount({
        worker: ["Mariah"],
        channel: ["slack"],
        status: ["pending"],
      }),
    ).toBe(3);
  });

  it("filter composes with empty serialization when all dimensions empty", () => {
    expect(serializeFilters(EMPTY_FILTERS)).toBe("");
    expect(parseFilters(new URLSearchParams(""))).toEqual(EMPTY_FILTERS);
  });
});
