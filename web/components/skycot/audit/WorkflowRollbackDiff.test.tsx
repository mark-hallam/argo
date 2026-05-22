import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { ActionSummaryHero } from "@/components/skycot/audit/ActionSummaryHero";
import { POLICY_SHOW_ALL } from "@/lib/redaction/redact";
import type { AuditDetail } from "@/lib/types/audit-detail";

/**
 * Covers the `workflow_rolled_back` branch of `ActionSummaryHero`
 * end-to-end: title line, step-count summary, expandable diff panel.
 *
 * Sample TOML pair encodes 1 added, 1 modified, 1 removed step so
 * the summary line can be asserted byte-precise — and adds 2 lines
 * of free-text drift outside step blocks so the LCS diff has visible
 * `+`/`-` markers when expanded.
 */

const BEFORE_TOML = `
name = "ai-receptionist"
version = 5

[[steps]]
name = "greet"
prompt = "Hello"

[[steps]]
name = "qualify"
prompt = "What can I help with?"

[[steps]]
name = "handoff_legacy"
prompt = "Routing to teammate."
`.trim();

const AFTER_TOML = `
name = "ai-receptionist"
version = 3

[[steps]]
name = "greet"
prompt = "Hello"

[[steps]]
name = "qualify"
prompt = "What brings you here today?"

[[steps]]
name = "handoff_modern"
prompt = "Routing to a teammate."
`.trim();

function buildRollbackDetail(overrides: Partial<AuditDetail> = {}): AuditDetail {
  const base: AuditDetail = {
    id: "act_01J0ROLLBACK",
    timestamp: "2026-05-21T10:30:00Z",
    worker: { id: "wkr_admin", name: "Admin" },
    actionType: "workflow_rolled_back",
    verb: "rolled back ai-receptionist workflow",
    status: "succeeded",
    approver: { id: "usr_op_mark", name: "Mark" },
    decider_role: "admin",
    inputs: {
      triggerKind: "message",
      triggerSummary: "Operator-initiated rollback.",
    },
    decisionChain: [
      { n: 1, title: "Memory loaded", body: "previous head + target version pulled" },
      { n: 8, title: "Outcome", body: "Rollback applied as additive new head." },
    ],
    provenance: {
      model: { id: "claude-opus-4-7", version: "2026-04-30" },
      promptTemplate: { id: "workflow.rollback", hash: "aa11bb22cc33dd44" },
      skill: { id: "workflow-mgr", version: "1.0.0" },
      memorySnapshot: { id: "mem_rollback" },
      policy: { id: "rollback-policy", version: "1.0", hash: "ee55ff66aa77bb88" },
    },
    output: {
      kind: "json",
      body: {
        vertical_id: "vert_ai_receptionist",
        workflow_name: "ai-receptionist",
        from_version: 5,
        to_version: 3,
        before_toml: BEFORE_TOML,
        after_toml: AFTER_TOML,
        decider_role: "admin",
        decider_name: "Mark",
      },
    },
  };
  return { ...base, ...overrides };
}

function renderHero(detail: AuditDetail) {
  return render(
    <ActionSummaryHero
      detail={detail}
      policy={POLICY_SHOW_ALL}
      onReplay={() => {}}
      onExport={() => {}}
    />,
  );
}

describe("ActionSummaryHero · workflow_rolled_back branch", () => {
  it("renders the rollback title, step-count summary, and a collapsed diff toggle", () => {
    renderHero(buildRollbackDetail());

    const block = screen.getByTestId("workflow-rollback-diff");
    expect(block).toHaveAttribute("data-from-version", "5");
    expect(block).toHaveAttribute("data-to-version", "3");
    expect(within(block).getByTestId("workflow-rollback-title")).toHaveTextContent(
      "Rolled back v5 → v3 by Admin Mark",
    );
    expect(within(block).getByTestId("workflow-rollback-summary")).toHaveTextContent(
      "3 steps changed, 1 added, 1 modified, 1 removed",
    );

    const toggle = within(block).getByTestId("workflow-rollback-toggle");
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(toggle).toHaveTextContent("Show diff");
    expect(screen.queryByTestId("workflow-rollback-diff-panel")).toBeNull();
  });

  it("expands to reveal LCS line-diff hunks with added/removed markers", async () => {
    const user = userEvent.setup();
    renderHero(buildRollbackDetail());

    await user.click(screen.getByTestId("workflow-rollback-toggle"));

    const panel = screen.getByTestId("workflow-rollback-diff-panel");
    expect(panel).toBeInTheDocument();
    const addedLine = within(panel)
      .getAllByText((_, node) => node?.getAttribute("data-kind") === "added")
      .map((n) => n.textContent ?? "");
    const removedLine = within(panel)
      .getAllByText((_, node) => node?.getAttribute("data-kind") === "removed")
      .map((n) => n.textContent ?? "");
    expect(addedLine.some((t) => t.includes('"handoff_modern"'))).toBe(true);
    expect(removedLine.some((t) => t.includes('"handoff_legacy"'))).toBe(true);
    expect(addedLine.some((t) => t.includes("What brings you here today?"))).toBe(true);
    expect(removedLine.some((t) => t.includes("What can I help with?"))).toBe(true);
  });

  it("uses the action's approver name when the payload omits decider_name", () => {
    const detail = buildRollbackDetail();
    detail.output = {
      ...detail.output,
      body: { ...(detail.output.body as Record<string, unknown>), decider_name: undefined },
    };
    renderHero(detail);

    expect(screen.getByTestId("workflow-rollback-title")).toHaveTextContent(
      "Rolled back v5 → v3 by Admin Mark",
    );
  });

  it("omits the diff toggle when before/after TOML bodies are missing", () => {
    const detail = buildRollbackDetail();
    detail.output = {
      ...detail.output,
      body: {
        vertical_id: "vert_ai_receptionist",
        from_version: 5,
        to_version: 3,
        decider_role: "admin",
        decider_name: "Mark",
      },
    };
    renderHero(detail);

    expect(screen.getByTestId("workflow-rollback-title")).toHaveTextContent(
      "Rolled back v5 → v3 by Admin Mark",
    );
    expect(screen.getByTestId("workflow-rollback-summary")).toHaveTextContent(
      "No steps changed.",
    );
    expect(screen.queryByTestId("workflow-rollback-toggle")).toBeNull();
  });

  it("does not render the rollback block for unrelated action types", () => {
    const detail = buildRollbackDetail({
      actionType: "candidate_screened",
      output: {
        kind: "json",
        body: { score: 0.93 },
      },
    });
    renderHero(detail);

    expect(screen.queryByTestId("workflow-rollback-diff")).toBeNull();
  });
});
