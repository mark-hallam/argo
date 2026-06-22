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

  it("enables Hire after confirm on a needs_review teammate (the operator confirmation IS the gate)", async () => {
    // A fresh Genesis teammate is ALWAYS `needs_review` (human-review-before-
    // activation by design, never `pass` at birth). The operator's confirm tick
    // is the activation gate; the gateway re-derives + checks the contract
    // digest server-side. Gating the button on `=== "pass"` left it permanently
    // disabled — the demo-blocker bug this test now pins against regressing.
    const user = userEvent.setup();
    const onConfirm = vi.fn();
    render(
      <RoleContractReview
        ready={ready({ judge_verdict: "needs_review" })}
        onConfirmHire={onConfirm}
        isHiring={false}
      />,
    );
    const button = screen.getByTestId("genesis-confirm-hire-button");
    // Before confirming, the button is still disabled — review can't be skipped.
    expect(button).toBeDisabled();
    // No "blocked / held" note for a needs_review teammate (only for `blocked`).
    expect(screen.queryByTestId("genesis-hire-held")).toBeNull();
    // Tick confirm → button enables → click fires the hire.
    await user.click(screen.getByTestId("genesis-confirm-checkbox"));
    expect(button).toBeEnabled();
    await user.click(button);
    expect(onConfirm).toHaveBeenCalledTimes(1);
  });

  it("ADVERSARIAL: a blocked verdict cannot be hired even when confirmed", async () => {
    const user = userEvent.setup();
    const onConfirm = vi.fn();
    render(
      <RoleContractReview
        ready={ready({ judge_verdict: "blocked" })}
        onConfirmHire={onConfirm}
        isHiring={false}
      />,
    );
    expect(screen.getByTestId("genesis-gate-badge")).toHaveTextContent(/Blocked/);
    const button = screen.getByTestId("genesis-confirm-hire-button");
    expect(button).toBeDisabled();
    // The blocked note explains the hard block.
    expect(screen.getByTestId("genesis-hire-held")).toHaveTextContent(/blocked/i);
    // Even after confirming, a hard-blocked teammate stays un-hireable —
    // confirming must NOT override the safety gate's block.
    await user.click(screen.getByTestId("genesis-confirm-checkbox"));
    expect(button).toBeDisabled();
    await user.click(button);
    expect(onConfirm).not.toHaveBeenCalled();
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

  it("DEC-048: surfaces cadence, exceptions, and typed success-criteria so the operator can catch any loss before hiring", () => {
    render(
      <RoleContractReview
        ready={ready({
          role_contract: contract({
            cadence: "every weekday at 5pm",
            exceptions: ["never touch legal escalations", "hand off refunds over $5k"],
            success_criteria: [
              { name: "on-time dispatch rate", kind: "percentage", target: 95, comparator: "gte" },
            ],
          }),
        })}
        onConfirmHire={() => {}}
        isHiring={false}
      />,
    );
    expect(screen.getByTestId("genesis-cadence")).toHaveTextContent("every weekday at 5pm");
    // A parseable cadence shows NO needs-confirmation note.
    expect(screen.queryByTestId("genesis-cadence-needs-confirm")).toBeNull();
    const ex = screen.getByTestId("genesis-exceptions");
    expect(ex).toHaveTextContent("never touch legal escalations");
    expect(ex).toHaveTextContent("hand off refunds over $5k");
    const kpi = screen.getByTestId("genesis-success-criteria");
    expect(kpi).toHaveTextContent("on-time dispatch rate");
    expect(kpi).toHaveTextContent(/95/);
    expect(kpi).toHaveTextContent(/percentage/);
  });

  it("DEC-048: an unparseable cadence shows the needs-confirmation note (no silent substitution)", () => {
    render(
      <RoleContractReview
        ready={ready({
          role_contract: contract({ cadence: "whenever an inbound ticket arrives" }),
        })}
        onConfirmHire={() => {}}
        isHiring={false}
      />,
    );
    expect(screen.getByTestId("genesis-cadence")).toHaveTextContent(
      "whenever an inbound ticket arrives",
    );
    expect(screen.getByTestId("genesis-cadence-needs-confirm")).toHaveTextContent(
      /needs confirmation/i,
    );
  });

  it("omits the intake panels entirely when no slots were captured", () => {
    render(<RoleContractReview ready={ready()} onConfirmHire={() => {}} isHiring={false} />);
    expect(screen.queryByTestId("genesis-cadence")).toBeNull();
    expect(screen.queryByTestId("genesis-exceptions")).toBeNull();
    expect(screen.queryByTestId("genesis-success-criteria")).toBeNull();
    expect(screen.queryByTestId("genesis-access-needs")).toBeNull();
  });

  it("DEC-048: surfaces the access the operator named at intake so it can't silently vanish before hire", () => {
    render(
      <RoleContractReview
        ready={ready({ role_contract: contract({ access_needs: ["Xero", "Slack"] }) })}
        onConfirmHire={() => {}}
        isHiring={false}
      />,
    );
    const panel = screen.getByTestId("genesis-access-needs");
    expect(panel).toHaveTextContent("Xero");
    expect(panel).toHaveTextContent("Slack");
    // And the Integrations card no longer contradicts the operator's intake ask —
    // it names the requested access as draft-only rather than a bare "None bound".
    expect(screen.queryByTestId("genesis-draft-integrations")).toBeNull();
    expect(screen.getByTestId("genesis-role-contract-review")).toHaveTextContent(
      /Xero, Slack were requested at intake/,
    );
  });

  it("omits the access-needs panel when none were captured (no empty contradiction)", () => {
    render(
      <RoleContractReview
        ready={ready({ role_contract: contract({ access_needs: [] }) })}
        onConfirmHire={() => {}}
        isHiring={false}
      />,
    );
    expect(screen.queryByTestId("genesis-access-needs")).toBeNull();
    // With no intake access, the Integrations card keeps the original copy.
    expect(screen.getByTestId("genesis-role-contract-review")).toHaveTextContent(
      /physically cannot\s+transmit through any live integration/,
    );
  });

  it("shows the hiring spinner state without re-firing the callback", () => {
    const onConfirm = vi.fn();
    render(<RoleContractReview ready={ready()} onConfirmHire={onConfirm} isHiring={true} />);
    const button = screen.getByTestId("genesis-confirm-hire-button");
    expect(button).toBeDisabled();
    expect(button).toHaveTextContent(/Hiring/);
  });

  it("⑤ readiness: lists what must be connected at hire (draft integrations ∪ intake access), so nothing stalls mid-task", () => {
    render(
      <RoleContractReview
        ready={ready({
          role_contract: contract({
            draft_integrations: ["gmail"],
            access_needs: ["Xero"],
          }),
        })}
        onConfirmHire={() => {}}
        isHiring={false}
      />,
    );
    const connect = screen.getByTestId("genesis-readiness-connect");
    expect(connect).toHaveTextContent("gmail");
    expect(connect).toHaveTextContent("Xero");
    // It frames the connect items as draft-but-can't-transmit (no overclaim).
    expect(connect).toHaveTextContent(/can't send or\s+transmit through it until you do/i);
    // With something to connect, it does NOT claim end-to-end readiness.
    expect(screen.queryByTestId("genesis-readiness-allclear")).toBeNull();
  });

  it("⑤ readiness: dedups a system captured both as draft integration AND intake access", () => {
    render(
      <RoleContractReview
        ready={ready({
          role_contract: contract({
            draft_integrations: ["gmail"],
            access_needs: ["Gmail"],
          }),
        })}
        onConfirmHire={() => {}}
        isHiring={false}
      />,
    );
    const items = screen.getByTestId("genesis-readiness-connect").querySelectorAll("li");
    // "gmail" and "Gmail" collapse to a single connect line.
    expect(items).toHaveLength(1);
  });

  it("⑤ readiness: with nothing to connect, says it can run end-to-end now — and names approvals as the only interruptions", () => {
    // The default contract has no draft integrations / access needs and a
    // parseable (absent) cadence, but DOES gate send_campaign on approval.
    render(<RoleContractReview ready={ready()} onConfirmHire={() => {}} isHiring={false} />);
    expect(screen.getByTestId("genesis-readiness-allclear")).toHaveTextContent(
      /run\s+end-to-end the moment you hire/i,
    );
    expect(screen.getByTestId("genesis-readiness-approvals")).toHaveTextContent("send_campaign");
    // No connect checklist when there's nothing to connect.
    expect(screen.queryByTestId("genesis-readiness-connect")).toBeNull();
  });

  it("⑤ readiness: an unparseable cadence becomes a clear-at-hire 'confirm the schedule' item, not a silent mid-task drop", () => {
    render(
      <RoleContractReview
        ready={ready({
          role_contract: contract({ cadence: "whenever an inbound ticket arrives" }),
        })}
        onConfirmHire={() => {}}
        isHiring={false}
      />,
    );
    expect(screen.getByTestId("genesis-readiness-schedule")).toHaveTextContent(
      /confirm the schedule/i,
    );
    // The presence of a to-clear item suppresses the all-clear message.
    expect(screen.queryByTestId("genesis-readiness-allclear")).toBeNull();
  });
});
