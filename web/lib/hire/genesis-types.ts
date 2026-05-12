/**
 * Types for the Step 2 "Listen" Genesis-lite classification flow.
 *
 * The classifier returns 1–3 clarifying questions, each with an
 * `answer_type` that dictates the answer surface (radio / checkbox /
 * textarea / yes-no toggle). v1 ships a permissive client; the
 * architectural-target hardening briefs (server-side moderation, cost
 * attribution, rate limiting) close via the Hardening backlog in the
 * E2-c brief.
 */

export type AnswerType = "single-select" | "multi-select" | "free-text" | "yes-no";

export interface Question {
  id: string;
  text: string;
  answer_type: AnswerType;
  /** Required for `single-select` / `multi-select`. */
  options?: string[];
  /** Defaults to `true`; Continue blocks until all required answers are filled. */
  required?: boolean;
}

export type QuestionAnswer =
  | { question_id: string; type: "single-select"; value: string }
  | { question_id: string; type: "multi-select"; values: string[] }
  | { question_id: string; type: "free-text"; value: string }
  | { question_id: string; type: "yes-no"; value: boolean };

export interface ClassifyResponse {
  classification?: string;
  questions: Question[];
}

/** Wire result of `useGenesisQuestions` — mirrors the brief's contract. */
export type GenesisStatus = "loading" | "ready" | "error" | "fallback";

/**
 * Returns whether a `QuestionAnswer` counts as filled. Used by the
 * Continue button to decide enabled-ness. Mirrors the rules in the
 * brief's "Continue validation" section.
 */
export function isAnswerFilled(answer: QuestionAnswer | undefined): boolean {
  if (!answer) return false;
  switch (answer.type) {
    case "single-select":
      return typeof answer.value === "string" && answer.value.length > 0;
    case "multi-select":
      return answer.values.length > 0;
    case "free-text":
      return answer.value.trim().length > 0;
    case "yes-no":
      return typeof answer.value === "boolean";
  }
}
