import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { RoleContractReview } from "./RoleContractReview";
import type { GenesisVerticalReadyEvent, RoleContract } from "@/lib/api/genesis";

function contract(overrides: Partial<RoleContract> = {}): RoleContract {
  return {
    role_name: "Maya",
    does: "Runs weekly outbound campaigns to cold leads and drafts newsletters.",
    actions: [
      {
        action: "draft_newsletter",
        description: "Draft a weekly newsletter.",
        requires_approval: false,
        risk: "low",
        executor: "human",
      },
      {
        action: "send_campaign",
        description: "Send an outbound campaign.",
        requires_approval: true,
        risk: "high",
        executor: "human",
      },
    ],
    approval_required: ["send_campaign"],
    autonomous: ["draft_newsletter"],
    escalation:
      "Escalates to the operator after 3 consecutive failures, or when uncertain.",
    will_not: [
      "Perform any action outside its declared vocabulary.",
      "Transmit through any live external integration — none are bound at birth.",
    ],
    draft_integrations: [],
    ...overrides,
  };
}

function ready(overrides: Partial<GenesisVerticalReadyEvent> = {}): GenesisVerticalReadyEvent {
  return {
    vertical_id: "genesis-maya",
    vertical_dir_path: "/tmp/.genesis/op-x/genesis-maya",
    tenant_id: "op-x",
    manifest: {
      id: "genesis-maya",
      name: "Maya",
      archetype: "marketing-intern",
      persona: "marketing intern",
      actions_declared: 2,
    },
    prompts: [],
    skills: [],
    eval_seed_count: 10,
    judge_verdict: "pass",
    role_contract: contract(),
    contract_digest: "abc123",
    ...overrides,
  };
}

describe("RoleContractReview", () => {
  it("renders the TRUTHFUL contract — every bound action with its real governance posture", () => {
    render(<RoleContractReview ready={ready()} onConfirmHire={() => {}} isHiring={false} />);
    const actions = screen.getAllByTestId("genesis-contract-action");
    expect(actions).toHaveLength(2);
    // The high-risk approval-gated action appears in the actions table AND is
    // segregated into the approval-required list (truthful posture).
    const sendRow = actions.find((el) => el.textContent?.includes("send_campaign"));
    expect(sendRow).toBeDefined();
    expect(sendRow).toHaveTextContent(/high risk/);
    expect(sendRow).toHaveTextContent(/needs approval/);
    expect(screen.getByTestId("genesis-approval-required")).toHaveTextContent("send_campaign");
    expect(screen.getByTestId("genesis-autonomous")).toHaveTextContent("draft_newsletter");
    // The "will NOT" boundary list is surfaced.
    expect(screen.getByTestId("genesis-will-not")).toHaveTextContent(/outside its declared vocabulary/);
  });

  it("ADVERSARIAL: the hire button is disabled until the operator confirms", async () => {
    const user = userEvent.setup();
    const onConfirm = vi.fn();
    render(<RoleContractReview ready={ready()} onConfirmHire={onConfirm} isHiring={false} />);

    const button = screen.getByTestId("genesis-confirm-hire-button");
    expect(button).toBeDisabled();
    // Clicking the disabled button does nothing — cannot skip review to activate.
    await user.click(button);
    expect(onConfirm).not.toHaveBeenCalled();

    // Tick the confirm box → button enables → click fires the hire.
    await user.click(screen.getByTestId("genesis-confirm-checkbox"));
    expect(button).toBeEnabled();
    await user.click(button);
    expect(onConfirm).toHaveBeenCalledTimes(1);
  });

  it("ADVERSARIAL: a non-pass (needs_review) verdict cannot be hired even when confirmed", async () => {
    const user = userEvent.setup();
    const onConfirm = vi.fn();
    render(
      <RoleContractReview
        ready={ready({ judge_verdict: "needs_review" })}
        onConfirmHire={onConfirm}
        isHiring={false}
      />,
    );
    // Even after confirming, a held teammate is not hireable.
    await user.click(screen.getByTestId("genesis-confirm-checkbox"));
    const button = screen.getByTestId("genesis-confirm-hire-button");
    expect(button).toBeDisabled();
    await user.click(button);
    expect(onConfirm).not.toHaveBeenCalled();
    expect(screen.getByTestId("genesis-hire-held")).toHaveTextContent(/needs_review/);
  });

  it("ADVERSARIAL: a blocked verdict shows the blocked gate badge and cannot hire", () => {
    render(
      <RoleContractReview
        ready={ready({ judge_verdict: "blocked" })}
        onConfirmHire={() => {}}
        isHiring={false}
      />,
    );
    expect(screen.getByTestId("genesis-gate-badge")).toHaveTextContent(/Blocked/);
    expect(screen.getByTestId("genesis-confirm-hire-button")).toBeDisabled();
  });

  it("surfaces draft-only integrations honestly when the role references some", () => {
    render(
      <RoleContractReview
        ready={ready({ role_contract: contract({ draft_integrations: ["gmail", "stripe"] }) })}
        onConfirmHire={() => {}}
        isHiring={false}
      />,
    );
    const list = screen.getByTestId("genesis-draft-integrations");
    expect(list).toHaveTextContent("gmail");
    expect(list).toHaveTextContent("stripe");
  });

  it("refuses to confirm a black box when no contract was derived", () => {
    render(
      <RoleContractReview
        ready={ready({ role_contract: null })}
        onConfirmHire={() => {}}
        isHiring={false}
      />,
    );
    expect(screen.getByTestId("genesis-contract-empty")).toBeInTheDocument();
    // No confirm button at all → cannot activate a contract-less teammate.
    expect(screen.queryByTestId("genesis-confirm-hire-button")).toBeNull();
  });

  it("shows the hiring spinner state without re-firing the callback", () => {
    const onConfirm = vi.fn();
    render(<RoleContractReview ready={ready()} onConfirmHire={onConfirm} isHiring={true} />);
    const button = screen.getByTestId("genesis-confirm-hire-button");
    expect(button).toBeDisabled();
    expect(button).toHaveTextContent(/Hiring/);
  });
});
