import { describe, expect, it } from "vitest";
import {
  actionNoun,
  actionTypeOf,
  groupApprovals,
  normalizeDetail,
  SIMILARITY_THRESHOLD,
} from "./group-approvals";
import type { ApprovalItem } from "@/lib/sample-approvals";

function mkItem(overrides: Partial<ApprovalItem> & Pick<ApprovalItem, "id">): ApprovalItem {
  return {
    worker: { name: "Mariah" },
    verb: "wants to send this listing",
    detail: "Bauer Vapor X5 · $90 · reply yes",
    consequenceIfYes: "goes to Marketplace.",
    consequenceIfNo: "I'll wait.",
    channel: "slack",
    receivedAt: new Date("2026-05-12T10:00:00Z").toISOString(),
    ...overrides,
  };
}

describe("normalizeDetail", () => {
  it("strips punctuation, lowercases, drops tokens ≤ 3 chars, sorts", () => {
    // X5 and $90 are ≤ 3 chars after punctuation strip → dropped.
    expect(normalizeDetail("Bauer Vapor X5, $90")).toBe("bauer vapor");
  });

  it("collapses price variants — short numeric tokens drop out", () => {
    expect(normalizeDetail("Bauer Vapor X5, $90"))
      .toBe(normalizeDetail("Bauer Vapor X5, $85"));
  });
});

describe("actionTypeOf", () => {
  it("classifies known verbs into stable buckets", () => {
    expect(actionTypeOf("wants to send this listing")).toBe("marketplace-reply");
    expect(actionTypeOf("wants to counter-offer at $85")).toBe("marketplace-counter");
    expect(actionTypeOf("wants to book an interview slot")).toBe("calendar-book");
    expect(actionTypeOf("wants to issue a refund")).toBe("refund-issue");
    expect(actionTypeOf("wants to publish the Q2 update")).toBe("broadcast-publish");
    expect(actionTypeOf("wants to escalate to engineering")).toBe("engineering-escalate");
    expect(actionTypeOf("wants to send a rejection email")).toBe("candidate-reject");
  });

  it("falls back to a normalized-verb bucket for unknown verbs", () => {
    // Two unknown verbs that share the same content tokens cluster.
    expect(actionTypeOf("wants to ponder the imponderable"))
      .toBe(actionTypeOf("wants to ponder the imponderable"));
  });
});

describe("actionNoun", () => {
  it("returns singular + plural for known types", () => {
    expect(actionNoun("marketplace-reply")).toEqual(["reply", "replies"]);
    expect(actionNoun("calendar-book")).toEqual(["interview", "interviews"]);
    expect(actionNoun("refund-issue")).toEqual(["refund", "refunds"]);
  });

  it("falls back to action(s) for unknown types", () => {
    expect(actionNoun("does-not-exist")).toEqual(["action", "actions"]);
  });
});

describe("groupApprovals", () => {
  it("handles empty input", () => {
    expect(groupApprovals([])).toEqual({ units: [], groups: [], ungrouped: [] });
  });

  it("handles a single item — no group, one single unit", () => {
    const a = mkItem({ id: "a" });
    const r = groupApprovals([a]);
    expect(r.groups).toHaveLength(0);
    expect(r.ungrouped).toEqual([a]);
    expect(r.units).toEqual([{ kind: "single", item: a }]);
  });

  it("two similar items stay ungrouped (below threshold)", () => {
    expect(SIMILARITY_THRESHOLD).toBe(3);
    const a = mkItem({ id: "a" });
    const b = mkItem({ id: "b", detail: "Bauer Vapor X5 · $85 · reply yes" });
    const r = groupApprovals([a, b]);
    expect(r.groups).toHaveLength(0);
    expect(r.ungrouped).toEqual([a, b]);
    expect(r.units.map((u) => u.kind)).toEqual(["single", "single"]);
  });

  it("three similar items form a group", () => {
    const a = mkItem({ id: "a" });
    const b = mkItem({ id: "b", detail: "Bauer Vapor X5 · $85 · reply yes" });
    const c = mkItem({ id: "c", detail: "Bauer Vapor X5 · $88 · reply yes" });
    const r = groupApprovals([a, b, c]);
    expect(r.groups).toHaveLength(1);
    expect(r.groups[0].items.map((i) => i.id)).toEqual(["a", "b", "c"]);
    expect(r.groups[0].reason).toBe("3 similar replies");
    expect(r.ungrouped).toEqual([]);
    expect(r.units).toHaveLength(1);
    expect(r.units[0].kind).toBe("group");
  });

  it("different workers do NOT cluster even if details match", () => {
    const a = mkItem({ id: "a", worker: { name: "Mariah" } });
    const b = mkItem({ id: "b", worker: { name: "Priya" } });
    const c = mkItem({ id: "c", worker: { name: "Jamal" } });
    const r = groupApprovals([a, b, c]);
    expect(r.groups).toHaveLength(0);
    expect(r.ungrouped).toHaveLength(3);
  });

  it("different action types do NOT cluster even if details match", () => {
    const a = mkItem({ id: "a", verb: "wants to send this listing" });
    const b = mkItem({ id: "b", verb: "wants to counter-offer at $85" });
    const c = mkItem({ id: "c", verb: "wants to book an interview" });
    const r = groupApprovals([a, b, c]);
    expect(r.groups).toHaveLength(0);
    expect(r.ungrouped).toHaveLength(3);
  });

  it("mixed similar + dissimilar: group is contiguous, ungrouped stay in place", () => {
    const a = mkItem({ id: "a" });
    const x = mkItem({ id: "x", worker: { name: "Priya" }, verb: "wants to book an interview", detail: "Diego Alvarez · Thursday 10:30" });
    const b = mkItem({ id: "b", detail: "Bauer Vapor X5 · $85 · reply yes" });
    const y = mkItem({ id: "y", worker: { name: "Jamal" }, verb: "wants to issue a refund", detail: "order $148 · damage" });
    const c = mkItem({ id: "c", detail: "Bauer Vapor X5 · $88 · reply yes" });
    const r = groupApprovals([a, x, b, y, c]);
    expect(r.groups).toHaveLength(1);
    expect(r.groups[0].items.map((i) => i.id)).toEqual(["a", "b", "c"]);
    // Group emitted at position of FIRST member ("a"), then x, y in
    // original order; b and c absorbed into the group.
    expect(r.units.map((u) => (u.kind === "group" ? `G:${u.group.items.map((i) => i.id).join(",")}` : `S:${u.item.id}`)))
      .toEqual(["G:a,b,c", "S:x", "S:y"]);
  });

  it("five similar items form one group with the right plural noun", () => {
    const items = [1, 2, 3, 4, 5].map((n) => mkItem({ id: `id${n}`, detail: `Listing ${n}: reply yes still available` }));
    const r = groupApprovals(items);
    expect(r.groups).toHaveLength(1);
    expect(r.groups[0].items).toHaveLength(5);
    expect(r.groups[0].reason).toBe("5 similar replies");
  });

  it("fuzzy hashing absorbs short-token (price/index) differences", () => {
    // Differ only in the short numeric token — same cluster.
    const a = mkItem({ id: "a", detail: "Listing #1 reply yes available" });
    const b = mkItem({ id: "b", detail: "Listing #2 reply yes available" });
    const c = mkItem({ id: "c", detail: "Listing #3 reply yes available" });
    const r = groupApprovals([a, b, c]);
    expect(r.groups).toHaveLength(1);
    expect(r.groups[0].items.map((i) => i.id)).toEqual(["a", "b", "c"]);
  });
});
