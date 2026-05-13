/**
 * Hand-crafted replay outcomes for the 5 canonical persona actions
 * (Priya / Sam / Mariah / Jamal / Nori) plus dedicated entries for
 * the decommissioned-model and side-effect-leak failure modes. Any
 * other action id falls through to `mockOutcomeForActionId` which
 * hashes the id to one of the three verdicts deterministically.
 *
 * These mocks are *visual fixtures*. The wire shape is the CONTRACT
 * the eventual server-side replay sandbox must emit; the field
 * values are tuned for the wireframe scenarios (reproducible drift,
 * skill-version-bump drift, policy-tightening misalignment).
 */
import type { ReplayOutcome } from "@/lib/api/replay";
import { mockVerdict } from "@/lib/api/replay";

/** Anchor id used by tests to exercise the model-decommissioned path. */
export const REPLAY_DECOMMISSIONED_ACTION_ID = "act_legacy_decommissioned";

/** Anchor id used by tests to exercise the side-effect leak path. */
export const REPLAY_LEAK_ACTION_ID = "act_leak_detected";

/** Anchor id used by tests to exercise the sandbox-failed path. */
export const REPLAY_FAILED_ACTION_ID = "act_sandbox_failed";

const PRIYA_001: ReplayOutcome = {
  kind: "success",
  verdict: "reproducible",
  result: {
    originalOutput: {
      kind: "json",
      body: { score: 0.93, mustHaves: { matched: 6, total: 6 }, recommendation: "advance" },
    },
    replayedOutput: {
      kind: "json",
      body: { score: 0.93, mustHaves: { matched: 6, total: 6 }, recommendation: "advance" },
    },
    provenanceDiff: {
      model: { changed: false },
      promptTemplate: { changed: false },
      skill: { changed: false },
      memorySnapshot: { changed: false },
      policy: { changed: false },
    },
  },
};

const SAM_001: ReplayOutcome = {
  kind: "success",
  verdict: "drifted-aligned",
  result: {
    originalOutput: {
      kind: "text",
      body:
        "Extracted 14 line items from INV-2024-08741.\nTotal: $14,250.\nMatched to PO-2024-1132.\nQueued for CFO sign-off.",
    },
    replayedOutput: {
      kind: "text",
      body:
        "Extracted 14 line items from INV-2024-08741.\nTotal: $14,250.\nMatched to PO-2024-1132.\nRouted to CFO via Slack (channel #ap-approvals).",
    },
    provenanceDiff: {
      model: { changed: false },
      promptTemplate: { changed: false },
      skill: { changed: true, from: "invoice-parse v3.1.0", to: "invoice-parse v3.1.4" },
      memorySnapshot: { changed: false },
      policy: { changed: false },
    },
  },
};

const MARIAH_001: ReplayOutcome = {
  kind: "success",
  verdict: "drifted-misaligned",
  result: {
    originalOutput: {
      kind: "text",
      body:
        "Re: RFP-2024-Q3-Tomás\n\nThank you for the opportunity. Our SLA: 99.95% with credit to 99.99% by Q1.",
    },
    replayedOutput: {
      kind: "text",
      body:
        "Re: RFP-2024-Q3-Tomás\n\nThank you for the opportunity. Our SLA: 99.99% guaranteed, with credit-extension auto-applied.",
    },
    provenanceDiff: {
      model: { changed: true, from: "claude-opus-4-7", to: "claude-opus-4-8" },
      promptTemplate: { changed: false },
      skill: { changed: false },
      memorySnapshot: { changed: false },
      policy: { changed: true, from: "contract-disclosure v3.0", to: "contract-disclosure v3.1" },
    },
  },
};

const JAMAL_001: ReplayOutcome = {
  kind: "unavailable",
  reason: "model-decommissioned",
  modelName: "claude-haiku-4-5",
};

const NORI_001: ReplayOutcome = {
  kind: "leak-detected",
  details:
    "Sandbox detected an outbound HTTP call to https://hooks.slack.com that was not stubbed by the trajectory recorder.",
};

const DECOMMISSIONED: ReplayOutcome = {
  kind: "unavailable",
  reason: "model-decommissioned",
  modelName: "claude-3-opus",
};

const LEAK: ReplayOutcome = {
  kind: "leak-detected",
  details: "Side-effect quarantine breach: MCP connector attempted live HTTP call.",
};

const FAILED: ReplayOutcome = {
  kind: "failed",
  reason: "Sandbox runner crashed (exit 137 — OOM).",
};

const TABLE: Record<string, ReplayOutcome> = {
  act_priya_001: PRIYA_001,
  act_sam_001: SAM_001,
  act_mariah_001: MARIAH_001,
  act_jamal_001: JAMAL_001,
  act_nori_001: NORI_001,
  [REPLAY_DECOMMISSIONED_ACTION_ID]: DECOMMISSIONED,
  [REPLAY_LEAK_ACTION_ID]: LEAK,
  [REPLAY_FAILED_ACTION_ID]: FAILED,
};

export function findSampleReplay(actionId: string): ReplayOutcome | undefined {
  return TABLE[actionId];
}

/** Fall-back when no hand-crafted sample matches: deterministic
 *  verdict from the action id. Synthesises an output diff + provenance
 *  diff appropriate to the verdict so the page always has something
 *  to render. */
export function mockOutcomeForActionId(actionId: string): ReplayOutcome {
  const verdict = mockVerdict(actionId);
  const baseOriginal = `Reconstructed original output for ${actionId}.`;
  switch (verdict) {
    case "reproducible":
      return {
        kind: "success",
        verdict,
        result: {
          originalOutput: { kind: "text", body: baseOriginal },
          replayedOutput: { kind: "text", body: baseOriginal },
          provenanceDiff: {
            model: { changed: false },
            promptTemplate: { changed: false },
            skill: { changed: false },
            memorySnapshot: { changed: false },
            policy: { changed: false },
          },
        },
      };
    case "drifted-aligned":
      return {
        kind: "success",
        verdict,
        result: {
          originalOutput: { kind: "text", body: baseOriginal },
          replayedOutput: {
            kind: "text",
            body: `${baseOriginal}\nSkill update produced a slightly different phrasing.`,
          },
          provenanceDiff: {
            model: { changed: false },
            promptTemplate: { changed: false },
            skill: { changed: true, from: "generic-skill v1.0.0", to: "generic-skill v1.0.2" },
            memorySnapshot: { changed: false },
            policy: { changed: false },
          },
        },
      };
    case "drifted-misaligned":
      return {
        kind: "success",
        verdict,
        result: {
          originalOutput: { kind: "text", body: baseOriginal },
          replayedOutput: {
            kind: "text",
            body: `${baseOriginal}\nPolicy tightening would block this output today.`,
          },
          provenanceDiff: {
            model: { changed: false },
            promptTemplate: { changed: false },
            skill: { changed: false },
            memorySnapshot: { changed: false },
            policy: { changed: true, from: "generic-policy v1.0", to: "generic-policy v1.1" },
          },
        },
      };
  }
}
