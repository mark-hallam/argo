import { describe, expect, it } from "vitest";
import type { AuditRow } from "@/lib/types/audit";
import {
  EMPTY_AUDIT_FILTERS,
  actionTypeLabel,
  activeAuditFilterCount,
  applyAuditMultiFilters,
  auditApproverOptions,
  auditActionTypeOptions,
  auditWorkerOptions,
  clearAuditDimension,
  inCostBand,
  isAuditFilterEmpty,
  parseAuditMultiFilters,
  serializeAuditMultiFilters,
  toggleAuditFilterValue,
} from "./audit";

function makeRow(overrides: Partial<AuditRow> = {}): AuditRow {
  return {
    id: "act_test",
    timestamp: "2026-05-13T10:00:00Z",
    worker: { id: "wkr_priya", name: "Priya" },
    actionType: "message_sent",
    verb: "did a thing",
    status: "succeeded",
    ...overrides,
  };
}

const sampleRows: AuditRow[] = [
  makeRow({
    id: "1",
    worker: { id: "wkr_priya", name: "Priya" },
    actionType: "candidate_screened",
    status: "succeeded",
    channel: "web",
    cost: { usd: 0.014 },
    approver: { id: "usr_op_mark", name: "Mark" },
    dataClassification: "confidential",
  }),
  makeRow({
    id: "2",
    worker: { id: "wkr_sam", name: "Sam" },
    actionType: "invoice_processed",
    status: "succeeded",
    channel: "slack",
    cost: { usd: 0 },
    approver: { id: "usr_cfo_jenna", name: "Jenna" },
    dataClassification: "internal",
  }),
  makeRow({
    id: "3",
    worker: { id: "wkr_mariah", name: "Mariah" },
    actionType: "rfp_drafted",
    status: "refused",
    channel: "slack",
    cost: { usd: 0.5 },
    approver: { id: "usr_compliance_aria", name: "Aria" },
    dataClassification: "restricted",
  }),
  makeRow({
    id: "4",
    worker: { id: "wkr_priya", name: "Priya" },
    actionType: "memory_written",
    status: "autonomous",
    cost: { usd: 0.05 },
    dataClassification: "public",
  }),
  makeRow({
    id: "5",
    worker: { id: "wkr_jamal", name: "Jamal" },
    actionType: "ticket_resolved",
    status: "failed",
    channel: "imessage",
    cost: { usd: 0.004 },
  }),
];

describe("audit filters", () => {
  it("empty filter passes everything through", () => {
    expect(isAuditFilterEmpty(EMPTY_AUDIT_FILTERS)).toBe(true);
    expect(applyAuditMultiFilters(sampleRows, EMPTY_AUDIT_FILTERS)).toEqual(
      sampleRows,
    );
  });

  it("worker dimension filters by worker.id", () => {
    const filters = { ...EMPTY_AUDIT_FILTERS, worker: ["wkr_priya"] };
    expect(applyAuditMultiFilters(sampleRows, filters).map((r) => r.id)).toEqual([
      "1",
      "4",
    ]);
  });

  it("channel dimension ORs within (slack | web) and drops rows without a channel", () => {
    const filters = {
      ...EMPTY_AUDIT_FILTERS,
      channel: ["slack" as const, "web" as const],
    };
    expect(applyAuditMultiFilters(sampleRows, filters).map((r) => r.id)).toEqual([
      "1",
      "2",
      "3",
    ]);
  });

  it("action type dimension matches the row's raw actionType string", () => {
    const filters = {
      ...EMPTY_AUDIT_FILTERS,
      actionType: ["rfp_drafted", "memory_written"],
    };
    expect(applyAuditMultiFilters(sampleRows, filters).map((r) => r.id)).toEqual([
      "3",
      "4",
    ]);
  });

  it("status dimension matches the row's status including autonomous", () => {
    expect(
      applyAuditMultiFilters(sampleRows, {
        ...EMPTY_AUDIT_FILTERS,
        status: ["autonomous"],
      }).map((r) => r.id),
    ).toEqual(["4"]);
    expect(
      applyAuditMultiFilters(sampleRows, {
        ...EMPTY_AUDIT_FILTERS,
        status: ["refused", "failed"],
      }).map((r) => r.id),
    ).toEqual(["3", "5"]);
  });

  it("approver dimension filters by approver.id and drops un-approved rows", () => {
    const filters = {
      ...EMPTY_AUDIT_FILTERS,
      approver: ["usr_op_mark", "usr_cfo_jenna"],
    };
    expect(applyAuditMultiFilters(sampleRows, filters).map((r) => r.id)).toEqual([
      "1",
      "2",
    ]);
  });

  it("cost-band bucketing covers the four documented bands", () => {
    expect(inCostBand(makeRow({ cost: { usd: 0 } }), "free")).toBe(true);
    expect(inCostBand(makeRow({ cost: undefined }), "free")).toBe(true);
    expect(inCostBand(makeRow({ cost: { usd: 0.004 } }), "lt_0_01")).toBe(true);
    expect(inCostBand(makeRow({ cost: { usd: 0.01 } }), "0_01_to_10")).toBe(true);
    expect(inCostBand(makeRow({ cost: { usd: 0.1 } }), "0_01_to_10")).toBe(true);
    expect(inCostBand(makeRow({ cost: { usd: 0.5 } }), "gt_0_10")).toBe(true);
    expect(inCostBand(makeRow({ cost: { usd: 0.5 } }), "free")).toBe(false);
  });

  it("cost-band dimension ORs across selected bands", () => {
    const filters = {
      ...EMPTY_AUDIT_FILTERS,
      costBand: ["free" as const, "gt_0_10" as const],
    };
    expect(applyAuditMultiFilters(sampleRows, filters).map((r) => r.id)).toEqual([
      "2",
      "3",
    ]);
  });

  it("classification dimension drops rows missing dataClassification", () => {
    const filters = {
      ...EMPTY_AUDIT_FILTERS,
      classification: ["confidential" as const, "restricted" as const],
    };
    expect(applyAuditMultiFilters(sampleRows, filters).map((r) => r.id)).toEqual([
      "1",
      "3",
    ]);
  });

  it("multi-dimension filter ANDs across dimensions", () => {
    const filters = {
      ...EMPTY_AUDIT_FILTERS,
      worker: ["wkr_priya"],
      status: ["succeeded" as const],
      channel: ["web" as const],
    };
    expect(applyAuditMultiFilters(sampleRows, filters).map((r) => r.id)).toEqual([
      "1",
    ]);
  });

  it("URL round-trip preserves all 7 dimensions", () => {
    const filters = {
      worker: ["wkr_priya", "wkr_sam"],
      channel: ["slack" as const, "web" as const],
      actionType: ["candidate_screened", "rfp_drafted"],
      status: ["succeeded" as const, "refused" as const],
      approver: ["usr_op_mark"],
      costBand: ["lt_0_01" as const, "gt_0_10" as const],
      classification: ["confidential" as const],
    };
    const qs = serializeAuditMultiFilters(filters);
    const parsed = parseAuditMultiFilters(new URLSearchParams(qs));
    expect(parsed.worker).toEqual(["wkr_priya", "wkr_sam"]);
    expect(parsed.channel).toEqual(["slack", "web"]);
    expect(parsed.actionType).toEqual(["candidate_screened", "rfp_drafted"]);
    expect(parsed.status).toEqual(["refused", "succeeded"]);
    expect(parsed.approver).toEqual(["usr_op_mark"]);
    expect(parsed.costBand).toEqual(["gt_0_10", "lt_0_01"]);
    expect(parsed.classification).toEqual(["confidential"]);
    expect(qs).toContain("action_type=");
    expect(qs).toContain("cost_band=");
    expect(qs).toContain("classification=");
  });

  it("parse drops unknown channel/status/cost_band/classification tokens silently", () => {
    const parsed = parseAuditMultiFilters(
      new URLSearchParams(
        "channel=slack,bogus&status=pending,gibberish&cost_band=free,nope&classification=public,nonsense",
      ),
    );
    expect(parsed.channel).toEqual(["slack"]);
    expect(parsed.status).toEqual(["pending"]);
    expect(parsed.costBand).toEqual(["free"]);
    expect(parsed.classification).toEqual(["public"]);
  });

  it("RBAC-gated classification dimension: rendering is the caller's job; parse passes through and apply still filters", () => {
    // The popover hides the dimension when the viewer lacks
    // unredacted-view, but a hand-typed URL bearing ?classification=
    // still parses + filters. Defense-in-depth lands when the
    // gateway enforces per-row redaction; v1 trusts the URL.
    const filters = parseAuditMultiFilters(
      new URLSearchParams("classification=restricted"),
    );
    expect(filters.classification).toEqual(["restricted"]);
    expect(applyAuditMultiFilters(sampleRows, filters).map((r) => r.id)).toEqual([
      "3",
    ]);
  });

  it("toggle adds and removes values within a dimension", () => {
    let f = EMPTY_AUDIT_FILTERS;
    f = toggleAuditFilterValue(f, "worker", "wkr_priya");
    expect(f.worker).toEqual(["wkr_priya"]);
    f = toggleAuditFilterValue(f, "worker", "wkr_sam");
    expect(f.worker).toEqual(["wkr_priya", "wkr_sam"]);
    f = toggleAuditFilterValue(f, "worker", "wkr_priya");
    expect(f.worker).toEqual(["wkr_sam"]);
  });

  it("clearDimension removes only the targeted dimension", () => {
    const f = {
      ...EMPTY_AUDIT_FILTERS,
      worker: ["wkr_priya"],
      channel: ["slack" as const],
      status: ["succeeded" as const],
    };
    const cleared = clearAuditDimension(f, "channel");
    expect(cleared.worker).toEqual(["wkr_priya"]);
    expect(cleared.channel).toEqual([]);
    expect(cleared.status).toEqual(["succeeded"]);
  });

  it("worker / approver / action-type option helpers derive alpha-sorted options from rows", () => {
    expect(auditWorkerOptions(sampleRows).map((w) => w.name)).toEqual([
      "Jamal",
      "Mariah",
      "Priya",
      "Sam",
    ]);
    expect(auditApproverOptions(sampleRows).map((a) => a.name)).toEqual([
      "Aria",
      "Jenna",
      "Mark",
    ]);
    const types = auditActionTypeOptions(sampleRows);
    expect(types).toContain("candidate_screened");
    expect(types).toContain("ticket_resolved");
    expect(types).toContain("message_sent");
    expect([...types].sort()).toEqual(types);
  });

  it("activeAuditFilterCount counts dimensions with at least one value", () => {
    expect(activeAuditFilterCount(EMPTY_AUDIT_FILTERS)).toBe(0);
    expect(
      activeAuditFilterCount({ ...EMPTY_AUDIT_FILTERS, worker: ["wkr_priya"] }),
    ).toBe(1);
    expect(
      activeAuditFilterCount({
        ...EMPTY_AUDIT_FILTERS,
        worker: ["wkr_priya"],
        channel: ["slack"],
        costBand: ["free"],
      }),
    ).toBe(3);
  });

  it("filter composes with empty serialization when all dimensions empty", () => {
    expect(serializeAuditMultiFilters(EMPTY_AUDIT_FILTERS)).toBe("");
    expect(parseAuditMultiFilters(new URLSearchParams(""))).toEqual(
      EMPTY_AUDIT_FILTERS,
    );
  });

  it("actionTypeLabel sentence-cases snake_case identifiers", () => {
    expect(actionTypeLabel("message_sent")).toBe("Message sent");
    expect(actionTypeLabel("rfp_drafted")).toBe("Rfp drafted");
    expect(actionTypeLabel("")).toBe("");
  });
});
