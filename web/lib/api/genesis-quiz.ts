/**
 * Genesis sandbox-quiz client — `POST /api/v1/genesis/quiz`.
 *
 * The "show me what you learned" gate before Hire (Phase 2): after the teammate
 * is drafted and documents are ingested, the operator quizzes the to-be-hired
 * teammate. It runs in a sandbox (no production tenant slot, no persistence
 * beyond the Genesis session) and answers by retrieving over the uploaded
 * documents — quoting the SOP it actually read.
 *
 * Real backend only — no stub fallback (operator memory `no-theater-in-demo`):
 * the answer the camera sees IS retrieved from the uploaded knowledge, and when
 * nothing matches the teammate says so plainly (`answered: false`) instead of
 * fabricating. Memory corrections are made by re-uploading a corrected document
 * via {@link ingestOnboardingDocuments}, then re-quizzing.
 */

import { apiPost } from "./client";

/** One cited passage backing an answer. */
export interface QuizCitation {
  source_label: string;
  excerpt: string;
}

/** The gateway's quiz response. */
export interface QuizResponse {
  /** `false` is the honest "no SOP covers this" path — never a fabrication. */
  answered: boolean;
  answer: string;
  citations: QuizCitation[];
  /** Grounded starter questions the operator can ask with one click. */
  suggested_questions: string[];
}

/**
 * Ask the sandbox teammate one question, retrieved over its uploaded knowledge.
 *
 * @param verticalId the `vertical_id` from the `vertical_ready` event.
 * @param question   the operator's free-form question.
 * @param role       optional role text used only to seed suggested questions.
 */
export async function askQuiz(
  verticalId: string,
  question: string,
  role?: string,
): Promise<QuizResponse> {
  return apiPost<QuizResponse>("/api/v1/genesis/quiz", {
    vertical_id: verticalId,
    question,
    role: role ?? "",
  });
}

/**
 * Prime the quiz panel: fetch the grounded suggested questions for a drafted
 * teammate without asking anything (no answer, no audit row). Sends an empty
 * `question`, which the gateway treats as a suggestions-only request.
 */
export async function fetchSuggestedQuestions(
  verticalId: string,
  role?: string,
): Promise<string[]> {
  const resp = await askQuiz(verticalId, "", role);
  return resp.suggested_questions;
}
