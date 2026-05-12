/**
 * Mock fixtures for the retrain flow. Kept out of `retrain.ts` so
 * the API client stays under its 250 LOC cap; this file is import-
 * only from the API layer and consumed via the gateway-404 fallback
 * path during v1 (sibling brief `mission-gateway-retrain-endpoints`
 * lands the real endpoints).
 */

import type {
  ClassifyRetrainRequest,
  CommitRetrainRequest,
  RetrainClassification,
  RetrainConflict,
  RetrainProposal,
  RetrainRecord,
  RetrainRegressionDiff,
} from "@/lib/api/retrain-types";

function hashString(s: string): number {
  let h = 0;
  for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) >>> 0;
  return h;
}

function classifyHeuristic(input: string): RetrainClassification {
  const lower = input.toLowerCase();
  if (/\b(never|don'?t|do not|stop|block|refuse|deny)\b/.test(lower)) return "action-ban";
  if (/\b(say|word|phrase|call|term|vocabulary|address them as)\b/.test(lower)) return "vocabulary";
  if (/\b(tone|formal|casual|warm|friendly|professional|voice|sound)\b/.test(lower)) return "skill";
  if (/\b(rule|policy|must|always|require|approval|sign[- ]?off)\b/.test(lower)) return "policy";
  return "memory";
}

function oneLineSummary(input: string, cls: RetrainClassification): string {
  const trimmed = input.trim().replace(/\s+/g, " ");
  const short = trimmed.length > 90 ? `${trimmed.slice(0, 87)}…` : trimmed;
  const verb: Record<RetrainClassification, string> = {
    memory: "Memory:",
    policy: "Policy:",
    skill: "Voice:",
    vocabulary: "Vocabulary:",
    "action-ban": "Action ban:",
  };
  return `${verb[cls]} ${short}`;
}

function plainEnglishFor(input: string, cls: RetrainClassification): string {
  const t = input.trim();
  switch (cls) {
    case "vocabulary":
      return `Going forward, she'll adjust her word choice: "${t}". She'll apply it across all outbound messages.`;
    case "policy":
      return `New rule: ${t} She'll check this rule before every applicable action.`;
    case "skill":
      return `Voice adjustment: ${t} She'll carry this tone across responses.`;
    case "action-ban":
      return `She'll no longer do the following: ${t} If asked, she'll politely refuse and escalate.`;
    case "memory":
    default:
      return `She'll remember: ${t} She'll bring this into context next time the situation comes up.`;
  }
}

const SAMPLE_CASES = [
  { caseId: "tr_1", context: "Inbound: 'Quick question about pricing'",
    newResponse: "Now applies the new rule when answering pricing questions." },
  { caseId: "tr_2", context: "Inbound: 'Can you confirm my appointment?'",
    newResponse: "Now applies the new rule before confirming." },
  { caseId: "tr_3", context: "Inbound: 'Refund please'",
    newResponse: "Now applies the new rule before issuing refunds." },
];

const REGRESSION_DIFFS: RetrainRegressionDiff[] = [
  { caseId: "rg_1", context: "Inbound: 'Can I get a discount?'",
    before: "I can offer 10% off if you sign today.",
    after: "I'd need to check with my manager on any discount.",
    outcome: "safe" },
  { caseId: "rg_2", context: "Inbound: 'Refund for order #9912'",
    before: "Approved — refund issued.",
    after: "Forwarded to Marta for sign-off.",
    outcome: "review" },
  { caseId: "rg_3", context: "Inbound: 'Ship to PO Box 4421'",
    before: "Shipping confirmed.",
    after: "Blocked — PO Box delivery not allowed.",
    outcome: "blocked" },
];

export function mockProposal(req: ClassifyRetrainRequest): RetrainProposal {
  const cls = classifyHeuristic(req.input);
  const h = hashString(req.input);
  const conflicts: RetrainConflict[] | undefined =
    h % 3 === 0
      ? [
          {
            conflictId: `cf_${h.toString(16).slice(0, 6)}`,
            existingRule:
              cls === "vocabulary"
                ? "Use 'customers' in outbound messages."
                : "Approve refunds under $50 automatically.",
            existingRuleAddedAt: new Date(Date.now() - 9 * 24 * 60 * 60_000).toISOString(),
            affectedCaseCount: 6 + (h % 5),
          },
        ]
      : undefined;
  const regressionPreview = h % 2 === 0 ? REGRESSION_DIFFS : undefined;
  return {
    classification: cls,
    plainEnglish: plainEnglishFor(req.input, cls),
    oneLineSummary: oneLineSummary(req.input, cls),
    conflicts,
    regressionPreview,
    structuralChange: { kind: `${cls}.update`, payload: { input: req.input } },
    trySamples: SAMPLE_CASES,
  };
}

export function mockRecord(req: CommitRetrainRequest): RetrainRecord {
  const now = new Date();
  return {
    retrainId: `rt_${hashString(JSON.stringify(req.proposal)).toString(16).slice(0, 8)}`,
    workerId: req.workerId,
    classification: req.proposal.classification,
    oneLineSummary: req.proposal.oneLineSummary,
    committedAt: now.toISOString(),
    undoExpiresAt: new Date(now.getTime() + 14 * 24 * 60 * 60_000).toISOString(),
  };
}

export const MOCK_REVERSE_CASES = [
  { caseId: "rv_1", context: "Inbound: 'PO Box 12 OK?'",
    before: "Shipping confirmed.", after: "Blocked — PO Box delivery not allowed." },
  { caseId: "rv_2", context: "Inbound: 'Express ship to PO Box'",
    before: "Quote sent.", after: "Blocked — PO Box delivery not allowed." },
  { caseId: "rv_3", context: "Inbound: 'Use PO Box 552 for return'",
    before: "Return label generated.", after: "Blocked — PO Box delivery not allowed." },
];
