/**
 * Pin: every action-id served by the worker Activity tab
 * (`/workers/<id>/activity`) resolves to a non-synthetic `AuditDetail`
 * via `findSampleAuditDetail`. Closes the demo-blocker where the
 * Activity tab's `act_<workerId>_NNN` IDs 404'd on drill-in.
 *
 * Companion to `sample-worker-overview.test.ts` (which pins the
 * Overview-tab activity card's IDs landing in the same lookup).
 */
import { describe, expect, it } from "vitest";
import {
  ACTIVITY_TAB_SEEDS,
  activityIdFor,
  findSampleWorkerActivity,
  SAMPLE_WORKER_ACTIVITY,
} from "@/lib/sample-worker-activity";
import { findSampleAuditDetail } from "@/lib/sample-audit-detail";
import { SAMPLE_PERSONAS } from "@/lib/sample-personas";

describe("worker Activity tab — IDs resolve through findSampleAuditDetail", () => {
  it("every activity row's id maps to a non-synthetic AuditDetail", () => {
    for (const [workerId, rows] of Object.entries(SAMPLE_WORKER_ACTIVITY)) {
      for (const row of rows) {
        const detail = findSampleAuditDetail(row.id);
        expect(detail, `no AuditDetail for ${row.id}`).toBeTruthy();
        expect(detail?.id, `id mismatch for ${row.id}`).toBe(row.id);
        expect(detail?.verb, `verb mismatch for ${row.id} (worker ${workerId})`).toBe(row.verb);
      }
    }
  });

  it("activityIdFor produces the canonical act_01J0<NAME>_0NN format", () => {
    expect(activityIdFor("PR", 0)).toBe("act_01J0PR_021");
    expect(activityIdFor("ma", 5)).toBe("act_01J0MA_026");
    expect(activityIdFor("JA", 8)).toBe("act_01J0JA_029");
  });

  it("Priya, Mariah, Jamal, Nori, Peta all have at least one activity row that resolves", () => {
    for (const personaId of Object.keys(SAMPLE_PERSONAS)) {
      const rows = findSampleWorkerActivity(personaId);
      if (rows.length === 0) continue;
      const detail = findSampleAuditDetail(rows[0]!.id);
      expect(detail, `first activity row for ${personaId} must resolve`).toBeTruthy();
    }
  });

  it("the seed counts match the rendered SAMPLE_WORKER_ACTIVITY counts (no drift)", () => {
    // If the build() function ever decouples from ACTIVITY_TAB_SEEDS,
    // this test will catch the drift before it ships broken drill-in.
    expect(SAMPLE_WORKER_ACTIVITY["018f67d0-0000-7000-8000-000000000002"]?.length).toBe(
      ACTIVITY_TAB_SEEDS.PRIYA.length,
    );
    expect(SAMPLE_WORKER_ACTIVITY["018f67d0-0000-7000-8000-000000000007"]?.length).toBe(
      ACTIVITY_TAB_SEEDS.PETA.length,
    );
    expect(SAMPLE_WORKER_ACTIVITY["018f67d0-0000-7000-8000-000000000001"]?.length).toBe(
      ACTIVITY_TAB_SEEDS.MARIAH.length,
    );
    expect(SAMPLE_WORKER_ACTIVITY["018f67d0-0000-7000-8000-000000000003"]?.length).toBe(
      ACTIVITY_TAB_SEEDS.JAMAL.length,
    );
    expect(SAMPLE_WORKER_ACTIVITY["018f67d0-0000-7000-8000-000000000004"]?.length).toBe(
      ACTIVITY_TAB_SEEDS.NORI.length,
    );
  });
});
