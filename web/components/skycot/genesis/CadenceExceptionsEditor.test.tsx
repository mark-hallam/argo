import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { CadenceExceptionsEditor } from "./CadenceExceptionsEditor";
import { buildHireDraftFromGenesis } from "@/lib/api/genesis-hire-intake";
import type { GenesisVerticalReadyEvent, RoleContract } from "@/lib/api/genesis";

function contract(over: Partial<RoleContract> = {}): RoleContract {
  return {
    role_name: "Support teammate",
    does: "Triage inbound tickets.",
    actions: [],
    approval_required: [],
    autonomous: [],
    escalation: "Escalate VIPs.",
    will_not: [],
    draft_integrations: [],
    cadence: "every weekday at 9am",
    exceptions: [],
    success_criteria: [],
    ...over,
  };
}

function ready(rc: RoleContract | null = contract()): GenesisVerticalReadyEvent {
  return {
    vertical_id: "support-teammate-sam",
    vertical_dir_path: ".genesis/op/support-teammate-sam",
    tenant_id: "op-test",
    manifest: {
      id: "support-teammate-sam",
      name: "Sam",
      archetype: "support",
      persona: "Calm support teammate.",
      actions_declared: 1,
    },
    prompts: [],
    skills: [],
    eval_seed_count: 2,
    judge_verdict: "pass",
    role_contract: rc,
    contract_digest: "sha256:abc",
  };
}

describe("CadenceExceptionsEditor", () => {
  it("renders nothing when the event has no role contract", () => {
    const { container } = render(
      <CadenceExceptionsEditor ready={ready(null)} onChange={vi.fn()} />,
    );
    expect(container).toBeEmptyDOMElement();
  });

  it("is collapsed by default and expands on the Adjust toggle", () => {
    render(<CadenceExceptionsEditor ready={ready()} onChange={vi.fn()} />);
    expect(screen.queryByTestId("genesis-cadence-input")).not.toBeInTheDocument();
    fireEvent.click(screen.getByTestId("genesis-cadence-editor-toggle"));
    // Pre-filled from the server contract.
    expect(screen.getByTestId("genesis-cadence-input")).toHaveValue(
      "every weekday at 9am",
    );
  });

  it("round-trips an edited cadence through buildHireDraftFromGenesis", () => {
    const onChange = vi.fn();
    render(<CadenceExceptionsEditor ready={ready()} onChange={onChange} />);
    fireEvent.click(screen.getByTestId("genesis-cadence-editor-toggle"));
    fireEvent.change(screen.getByTestId("genesis-cadence-input"), {
      target: { value: "every morning at 7am" },
    });

    // The editor emits a NEW event with the edit merged into role_contract.
    const emitted = onChange.mock.calls.at(-1)?.[0] as GenesisVerticalReadyEvent;
    expect(emitted.role_contract?.cadence).toBe("every morning at 7am");

    // …and that event builds a hire draft carrying the edited value.
    const draft = buildHireDraftFromGenesis(emitted, "Handle support tickets.");
    expect(draft.cadence).toBe("every morning at 7am");
  });

  it("falls back to the server cadence when the operator clears the field", () => {
    const onChange = vi.fn();
    render(<CadenceExceptionsEditor ready={ready()} onChange={onChange} />);
    fireEvent.click(screen.getByTestId("genesis-cadence-editor-toggle"));
    fireEvent.change(screen.getByTestId("genesis-cadence-input"), {
      target: { value: "" },
    });
    const emitted = onChange.mock.calls.at(-1)?.[0] as GenesisVerticalReadyEvent;
    // Empty edit → server default, never a divergent blank.
    expect(emitted.role_contract?.cadence).toBe("every weekday at 9am");
  });
});
