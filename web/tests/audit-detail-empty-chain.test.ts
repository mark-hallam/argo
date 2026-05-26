import { render, screen } from "@testing-library/react";
import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { DecisionChainCard } from "@/components/skycot/audit/DecisionChainCard";
import { ProvenanceBlock } from "@/components/skycot/audit/ProvenanceBlock";
import { POLICY_SHOW_ALL } from "@/lib/redaction/redact";
import type { AuditDetail, DecisionStep } from "@/lib/types/audit-detail";

function detailFixture(overrides: Partial<AuditDetail> = {}): AuditDetail {
  return {
    id: "audit_meta_1",
    timestamp: "2026-05-26T02:50:00.000Z",
    worker: { id: "worker_audit", name: "Audit trail" },
    actionType: "audit.detail.viewed",
    verb: "viewed audit detail",
    status: "succeeded",
    dataClassification: "internal",
    inputs: {
      triggerKind: "message",
      triggerSummary: "Operator opened an audit detail row.",
    },
    decisionChain: [],
    provenance: {
      model: { id: "", version: "" },
      promptTemplate: { id: "", hash: "" },
      skill: { id: "", version: "" },
      memorySnapshot: { id: "" },
      policy: { id: "", version: "", hash: "" },
    },
    output: { kind: "text", body: "Viewed audit detail." },
    ...overrides,
  };
}

function hollowMetaChain(): DecisionStep[] {
  return [
    { n: 1, title: "Memory loaded", body: "", details: { snapshot_id: null } },
    { n: 2, title: "Skill chosen", body: "", details: { skill_id: null, skill_version: null } },
    { n: 3, title: "Plan generated", body: "", details: { plan_id: null } },
    { n: 4, title: "Plan selected", body: "", details: { score: null } },
    { n: 5, title: "Policy check", body: "", details: { policy_id: null } },
    { n: 6, title: "Approval", body: "", details: { approver_id: null } },
    { n: 7, title: "Execution", body: "", details: { executor_id: null } },
    { n: 8, title: "Outcome", body: "", details: { receipt_id: null } },
  ];
}

function partialAgentChain(): DecisionStep[] {
  return [
    { n: 1, title: "Memory loaded", body: "snapshot abc · customer terms", details: { snapshot_id: "abc" } },
    { n: 2, title: "Skill chosen", body: "review-skill v3 selected", details: { skill_id: "review-skill", skill_version: "3" } },
    { n: 3, title: "Plan generated", body: "Draft response and request approval." },
    { n: 4, title: "Plan selected", body: "", details: { score: null } },
    { n: 5, title: "Policy check", body: "", details: { policy_id: null } },
    { n: 6, title: "Approval", body: "", details: { approver_id: null } },
    { n: 7, title: "Execution", body: "", details: { executor_id: null } },
    { n: 8, title: "Outcome", body: "", details: { receipt_id: null } },
  ];
}

describe("audit detail empty chain fallbacks", () => {
  it("renders empty states for meta-events instead of hollow chain and provenance rows", () => {
    const metaDetail = detailFixture({ decisionChain: hollowMetaChain() });

    render(
      createElement(DecisionChainCard, {
        policy: POLICY_SHOW_ALL,
        steps: metaDetail.decisionChain,
      }),
    );
    expect(screen.getByTestId("audit-decision-chain-empty")).toHaveTextContent(
      "Decision chain unavailable",
    );
    expect(screen.getByTestId("audit-decision-chain-empty")).toHaveTextContent(
      /meta-event, not an agent turn/i,
    );
    expect(screen.queryByTestId("audit-decision-step-1")).not.toBeInTheDocument();

    render(
      createElement(ProvenanceBlock, {
        data: metaDetail.provenance,
        detail: metaDetail,
      }),
    );
    expect(screen.getByTestId("audit-provenance-empty")).toHaveTextContent(
      "Provenance unavailable",
    );
    expect(screen.getByTestId("audit-provenance-empty")).toHaveTextContent(
      /audit\.detail\.viewed row is a meta-event/i,
    );
    expect(screen.queryByTestId("audit-provenance-row-model")).not.toBeInTheDocument();
  });

  it("renders populated agent-turn stages and marks the rest as not reached", () => {
    render(
      createElement(DecisionChainCard, {
        policy: POLICY_SHOW_ALL,
        steps: partialAgentChain(),
      }),
    );

    expect(screen.queryByTestId("audit-decision-chain-empty")).not.toBeInTheDocument();
    expect(screen.getByTestId("audit-decision-step-1-body")).toHaveTextContent(
      "snapshot abc",
    );
    expect(screen.getByTestId("audit-decision-step-2-body")).toHaveTextContent(
      "review-skill v3 selected",
    );
    expect(screen.getByTestId("audit-decision-step-3-body")).toHaveTextContent(
      "Draft response",
    );
    expect(screen.getAllByText("Not reached")).toHaveLength(5);
    expect(screen.getByTestId("audit-decision-step-4-body")).toHaveTextContent(
      "Not reached",
    );
    expect(screen.queryByTestId("audit-decision-step-4-toggle")).not.toBeInTheDocument();
  });
});
