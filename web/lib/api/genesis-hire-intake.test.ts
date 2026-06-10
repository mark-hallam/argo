import { describe, it, expect } from "vitest";
import {
  applyIntakeEdits,
  buildHireDraftFromGenesis,
  intakeFromContract,
} from "./genesis-hire-intake";
import type { GenesisVerticalReadyEvent, RoleContract } from "./genesis";

function contract(over: Partial<RoleContract> = {}): RoleContract {
  return {
    role_name: "Support teammate",
    does: "Triage inbound tickets.",
    actions: [],
    approval_required: [],
    autonomous: [],
    escalation: "Escalate VIPs to a human.",
    will_not: [],
    draft_integrations: [],
    cadence: "every weekday at 9am",
    exceptions: ["never touch legal escalations"],
    success_criteria: [
      { name: "first response time", kind: "duration_ms", target: 300000, comparator: "lte" },
    ],
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
      persona: "Calm, precise support teammate.",
      actions_declared: 3,
    },
    prompts: [],
    skills: [],
    eval_seed_count: 4,
    judge_verdict: "pass",
    role_contract: rc,
    contract_digest: "sha256:abc",
  };
}

describe("intakeFromContract", () => {
  it("seeds the selection from the server contract", () => {
    expect(intakeFromContract(contract())).toEqual({
      cadence: "every weekday at 9am",
      exceptions: ["never touch legal escalations"],
      success_criteria: [
        { name: "first response time", kind: "duration_ms", target: 300000, comparator: "lte" },
      ],
    });
  });

  it("returns the empty selection for a null contract", () => {
    expect(intakeFromContract(null)).toEqual({
      cadence: "",
      exceptions: [],
      success_criteria: [],
    });
  });
});

describe("applyIntakeEdits", () => {
  it("overrides the cadence with the operator's edit", () => {
    const merged = applyIntakeEdits(ready(), {
      cadence: "every morning at 7am",
      exceptions: [],
      success_criteria: [],
    });
    expect(merged.role_contract?.cadence).toBe("every morning at 7am");
    // Untouched slots fall back to the server echo.
    expect(merged.role_contract?.exceptions).toEqual(["never touch legal escalations"]);
    // The governance digest is preserved → confirmation still matches.
    expect(merged.contract_digest).toBe("sha256:abc");
  });

  it("falls back to server defaults when an edit is blank", () => {
    const merged = applyIntakeEdits(ready(), {
      cadence: "   ",
      exceptions: [],
      success_criteria: [],
    });
    expect(merged.role_contract?.cadence).toBe("every weekday at 9am");
  });

  it("is a no-op when there is no role contract", () => {
    const base = ready(null);
    expect(applyIntakeEdits(base, intakeFromContract(null))).toBe(base);
  });
});

describe("buildHireDraftFromGenesis round-trip", () => {
  it("carries the EDITED cadence into the hire draft", () => {
    // Round-trip: server contract → operator edit → merged event → draft.
    const merged = applyIntakeEdits(ready(), {
      cadence: "every morning at 7am",
      exceptions: ["skip duplicate alerts"],
      success_criteria: [],
    });
    const draft = buildHireDraftFromGenesis(merged, "Handle support tickets for me.");
    expect(draft.suggestedName).toBe("Sam");
    expect(draft.cadence).toBe("every morning at 7am");
    expect(draft.exceptions).toEqual(["skip duplicate alerts"]);
    // Unedited success-criteria still ride along from the server echo.
    expect(draft.successCriteria).toEqual([
      { name: "first response time", kind: "duration_ms", target: 300000, comparator: "lte" },
    ]);
  });

  it("omits intake slots entirely when none were captured", () => {
    const draft = buildHireDraftFromGenesis(
      ready(contract({ cadence: null, exceptions: [], success_criteria: [] })),
      "Handle support tickets for me.",
    );
    expect(draft.cadence).toBeUndefined();
    expect(draft.exceptions).toBeUndefined();
    expect(draft.successCriteria).toBeUndefined();
    // The required validator fields are still present.
    expect(draft.suggestedVertical).toBe("support-teammate-sam");
    expect(draft.genesisPlanId).toBe("support-teammate-sam");
  });
});
