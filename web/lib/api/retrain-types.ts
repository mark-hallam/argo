/**
 * Public types for the retrain flow. Kept separate from `retrain.ts`
 * to leave room under the API client's LOC cap and so that mocks
 * and hooks can import the types without pulling the gateway client.
 */

export type RetrainSource = "gateway" | "mock";

export type RetrainClassification =
  | "memory"
  | "policy"
  | "skill"
  | "vocabulary"
  | "action-ban";

export interface RetrainConflict {
  conflictId: string;
  existingRule: string;
  existingRuleAddedAt: string;
  affectedCaseCount: number;
}

export interface RetrainRegressionDiff {
  caseId: string;
  context: string;
  before: string;
  after: string;
  outcome: "safe" | "review" | "blocked";
}

export interface RetrainProposal {
  classification: RetrainClassification;
  plainEnglish: string;
  oneLineSummary: string;
  conflicts?: RetrainConflict[];
  regressionPreview?: RetrainRegressionDiff[];
  structuralChange: {
    kind: string;
    payload: Record<string, unknown>;
  };
  trySamples?: Array<{ caseId: string; context: string; newResponse: string }>;
}

export interface ClassifyRetrainRequest {
  workerId: string;
  input: string;
  triggerContext: "charter" | "voice" | "audit";
}

export interface ClassifyRetrainResult {
  proposal: RetrainProposal;
  source: RetrainSource;
}

export interface CommitRetrainRequest {
  workerId: string;
  proposal: RetrainProposal;
  /** Per-regression-case accept overrides; defaults to all "safe" auto-accepted. */
  acceptedCaseIds?: string[];
}

export interface RetrainRecord {
  retrainId: string;
  workerId: string;
  classification: RetrainClassification;
  oneLineSummary: string;
  committedAt: string;
  undoExpiresAt: string;
  undoneAt?: string;
}

export interface CommitRetrainResult {
  record: RetrainRecord;
  source: RetrainSource;
}

export interface ReverseTestCase {
  caseId: string;
  context: string;
  before: string;
  after: string;
}

export interface ReverseTestResult {
  cases: ReverseTestCase[];
  source: RetrainSource;
}

export interface ListRecentRetrainsResult {
  records: RetrainRecord[];
  source: RetrainSource;
}
