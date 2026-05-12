import type { Question } from "./genesis-types";

/**
 * Client-side fallback questions surfaced when the gateway is
 * unreachable. They are intentionally generic — the architectural
 * target is server-side Genesis with no client fallback; this stub
 * exists so the hire flow doesn't dead-end when the backend is down.
 *
 * Per the brief, the stub does NOT specialize to the user's input.
 * Trying to look clever here would just paper over the missing
 * backend and dilute the signal when Genesis fails.
 */
export const STUB_QUESTIONS: readonly Question[] = [
  {
    id: "stub_action_mode",
    text: "Should they reply on your behalf, or draft replies for you to send?",
    answer_type: "single-select",
    options: ["Reply on my behalf", "Draft for me to send", "Let me decide later"],
    required: true,
  },
  {
    id: "stub_autonomy",
    text: "Should they ever take an action without asking you?",
    answer_type: "yes-no",
    required: true,
  },
  {
    id: "stub_name",
    text: "What name should they use?",
    answer_type: "free-text",
    required: true,
  },
] as const;

/** Banner copy surfaced above stub questions so users know it's a fallback. */
export const STUB_BANNER_MESSAGE =
  "Backup questions — your full setup will happen on the next screen.";
