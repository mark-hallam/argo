import type { Question } from "./genesis-types";
import type { WorkerDraft } from "./draft-types";

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

/**
 * Client-side fallback worker draft surfaced when the gateway
 * `/api/v1/genesis/draft-worker` endpoint is unreachable (404 — the
 * sibling brief that lands the server-side endpoint has not merged
 * yet). Intentionally generic — a marketplace-seller-shaped draft that
 * works as a starting point for any input; the user adjusts in Step 4
 * before commit. The architectural target is server-side Genesis with
 * no client fallback (see `mission-enterprise-genesis-draft-mandatory`).
 *
 * The shape is required to be a valid `WorkerDraft` — Step 4 onward
 * depends on all fields being populated.
 */
export function stubWorkerDraft(_problem: string): WorkerDraft {
  return {
    suggestedName: "Mariah",
    persona: {
      tagline: "Marketplace assistant",
      avatarKind: "initials",
      voiceProfile: {
        tone: "warm",
        formality: "medium",
        signaturePhrases: ["Let me check on that for you.", "Sounds good!"],
      },
    },
    charter:
      "Mariah helps you manage your inbound customer messages. She drafts replies in your voice and asks you before taking unusual actions.",
    duties: [
      {
        label: "Morning summary of overnight messages",
        schedule: { kind: "cron", spec: "0 8 * * *" },
        defaultEnabled: true,
      },
      {
        label: "Draft replies to inbound messages",
        schedule: { kind: "event", spec: "message.inbound" },
        defaultEnabled: true,
      },
      {
        label: "Evening check-in with the day's outstanding items",
        schedule: { kind: "cron", spec: "0 18 * * *" },
        defaultEnabled: false,
      },
    ],
    exampleReplies: [
      {
        inboundContext: "Is this still available?",
        mariahResponse:
          "Yes, it's still available! Would you like to come pick it up this week?",
      },
      {
        inboundContext: "Will you take $40?",
        mariahResponse:
          "Thanks for the offer — let me check with the owner and get back to you shortly.",
      },
      {
        inboundContext: "Can you ship it?",
        mariahResponse:
          "I can check on shipping options for you — what's your zip code?",
      },
    ],
    approvalPolicy: { level: "balanced" },
    suggestedVertical: "marketplace-seller",
  };
}
