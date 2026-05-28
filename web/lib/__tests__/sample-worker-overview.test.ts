import { describe, expect, it } from "vitest";
import {
  findSampleWorkerOverview,
  SAMPLE_WORKER_OVERVIEW_IDS,
} from "@/lib/sample-worker-overview";
import {
  findSampleAuditDetail,
  placeholderAuditDetail,
} from "@/lib/sample-audit-detail";

describe("sample worker-overview activity hrefs resolve", () => {
  it("every recentActivity row id resolves to a non-placeholder audit detail", () => {
    const placeholderVerb = placeholderAuditDetail("__sentinel__").verb;
    const unresolved: Array<{ workerId: string; rowId: string }> = [];
    const placeholderHit: Array<{ workerId: string; rowId: string }> = [];

    for (const workerId of SAMPLE_WORKER_OVERVIEW_IDS) {
      const overview = findSampleWorkerOverview(workerId);
      expect(overview, `missing overview for ${workerId}`).toBeDefined();
      for (const row of overview!.recentActivity) {
        const detail = findSampleAuditDetail(row.id);
        if (!detail) {
          unresolved.push({ workerId, rowId: row.id });
          continue;
        }
        if (detail.verb === placeholderVerb) {
          placeholderHit.push({ workerId, rowId: row.id });
        }
      }
    }

    expect(unresolved).toEqual([]);
    expect(placeholderHit).toEqual([]);
  });

  it("every recentActivity row href starts with /audit/action/ and matches its id", () => {
    for (const workerId of SAMPLE_WORKER_OVERVIEW_IDS) {
      const overview = findSampleWorkerOverview(workerId);
      for (const row of overview!.recentActivity) {
        expect(row.href).toBe(`/audit/action/${row.id}`);
      }
    }
  });

  it("resolved detail row id, verb, and worker name agree with the activity-card row", () => {
    for (const workerId of SAMPLE_WORKER_OVERVIEW_IDS) {
      const overview = findSampleWorkerOverview(workerId);
      for (const row of overview!.recentActivity) {
        const detail = findSampleAuditDetail(row.id);
        expect(detail?.id).toBe(row.id);
        expect(detail?.verb).toBe(row.verb);
      }
    }
  });
});
