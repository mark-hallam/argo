import { describe, expect, it, vi } from "vitest";
import { QueryClient } from "@tanstack/react-query";
import { applyEvent, wireToApprovalItem } from "./use-approvals-live";
import type { ApprovalsResult } from "@/lib/api/approvals";
import type { ApprovalItem } from "@/lib/sample-approvals";
import type { WireApprovalCreatedPayload } from "@/lib/api/types";

const APPROVALS_KEY = ["approvals"] as const;

function makeQc(initial?: ApprovalsResult): QueryClient {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  if (initial) qc.setQueryData<ApprovalsResult>([...APPROVALS_KEY], initial);
  return qc;
}

function fullPayload(over: Partial<WireApprovalCreatedPayload> = {}): WireApprovalCreatedPayload {
  return {
    approval_id: "ap_new",
    id: "ap_new",
    worker: { id: "ap_new", name: "ap_new" },
    verb: "wants to act",
    detail: "Approve archival",
    consequence_if_yes: "",
    consequence_if_no: "",
    reason: null,
    channel: "web",
    risk_level: "high",
    received_at: "2026-05-12T10:00:00Z",
    expires_at: "2026-05-12T10:05:00Z",
    ...over,
  };
}

function existingItem(id: string, name = "worker-x"): ApprovalItem {
  return {
    id,
    worker: { name },
    verb: "wants to do thing",
    detail: "Existing approval",
    consequenceIfYes: "yes",
    consequenceIfNo: "no",
    channel: "web",
    receivedAt: "2026-05-12T09:55:00Z",
    decision: null,
  };
}

describe("useApprovalsLive applyEvent", () => {
  it("(a) surgical insert keeps existing items and prepends the new row", () => {
    const initial: ApprovalsResult = {
      items: [existingItem("ap_keep_a"), existingItem("ap_keep_b")],
      source: "live",
    };
    const qc = makeQc(initial);
    const invalidate = vi.spyOn(qc, "invalidateQueries");
    applyEvent(qc, { type: "approval.created", payload: fullPayload({ id: "ap_new", approval_id: "ap_new" }) });
    const next = qc.getQueryData<ApprovalsResult>([...APPROVALS_KEY])!;
    expect(next.items.map((i) => i.id)).toEqual(["ap_new", "ap_keep_a", "ap_keep_b"]);
    expect(next.items[0]?.detail).toBe("Approve archival");
    expect(invalidate).not.toHaveBeenCalled();
  });

  it("(b) partial fallback invalidates rather than splicing", () => {
    const qc = makeQc({ items: [existingItem("ap_keep")], source: "live" });
    const invalidate = vi.spyOn(qc, "invalidateQueries").mockResolvedValue();
    applyEvent(qc, {
      type: "approval.created",
      payload: {
        approval_id: "ap_partial",
        partial: true,
        prompt_body: "anything",
        risk_level: "medium",
        enqueued_at: "2026-05-12T10:00:00Z",
        expiry: "2026-05-12T10:05:00Z",
      },
    });
    expect(invalidate).toHaveBeenCalledWith({ queryKey: APPROVALS_KEY });
    const next = qc.getQueryData<ApprovalsResult>([...APPROVALS_KEY])!;
    expect(next.items.map((i) => i.id)).toEqual(["ap_keep"]);
  });

  it("(c) dedupes when approval_id is already in the cache", () => {
    const initial: ApprovalsResult = {
      items: [existingItem("ap_dup")],
      source: "live",
    };
    const qc = makeQc(initial);
    applyEvent(qc, { type: "approval.created", payload: fullPayload({ id: "ap_dup", approval_id: "ap_dup" }) });
    const next = qc.getQueryData<ApprovalsResult>([...APPROVALS_KEY])!;
    expect(next.items.map((i) => i.id)).toEqual(["ap_dup"]);
    expect(next.items[0]?.detail).toBe("Existing approval");
  });

  it("(d) wrong-shape payload falls back to invalidate instead of crashing", () => {
    const qc = makeQc({ items: [existingItem("ap_keep")], source: "live" });
    const invalidate = vi.spyOn(qc, "invalidateQueries").mockResolvedValue();
    applyEvent(qc, {
      type: "approval.created",
      payload: {
        approval_id: "ap_bad",
        id: "ap_bad",
        worker: undefined,
        verb: "wants to act",
      } as unknown as WireApprovalCreatedPayload,
    });
    expect(invalidate).toHaveBeenCalledWith({ queryKey: APPROVALS_KEY });
    const next = qc.getQueryData<ApprovalsResult>([...APPROVALS_KEY])!;
    expect(next.items.map((i) => i.id)).toEqual(["ap_keep"]);
  });
});

describe("wireToApprovalItem", () => {
  it("maps the full wire shape onto the camelCase ApprovalItem", () => {
    const item = wireToApprovalItem(
      fullPayload({
        id: "ap_full",
        approval_id: "ap_full",
        worker: { id: "w-1", name: "Mariah", avatar_url: "http://avatar/x" },
        consequence_if_yes: "ship it",
        consequence_if_no: "drop it",
        reason: "manifest stamp",
      }),
    );
    expect(item).toEqual({
      id: "ap_full",
      worker: { name: "Mariah", avatarUrl: "http://avatar/x" },
      verb: "wants to act",
      detail: "Approve archival",
      consequenceIfYes: "ship it",
      consequenceIfNo: "drop it",
      reason: "manifest stamp",
      channel: "web",
      receivedAt: "2026-05-12T10:00:00Z",
      decision: null,
    });
  });

  it("returns null on partial payloads", () => {
    expect(
      wireToApprovalItem({ approval_id: "ap_partial", partial: true }),
    ).toBeNull();
  });

  it("falls back to 'web' channel for unknown channel ids", () => {
    const item = wireToApprovalItem(fullPayload({ channel: "smoke-signal" }));
    expect(item?.channel).toBe("web");
  });
});
