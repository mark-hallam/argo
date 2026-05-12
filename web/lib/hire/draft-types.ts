/**
 * Shape of an in-flight hire draft persisted across reloads.
 *
 * The hire flow has 6 user-facing steps (Step 1–6); the draft tracks
 * which step the user reached. Per-step body fields are added as
 * downstream E2 briefs implement each step's content; the v1 shell
 * stores them under an opaque `answers` map plus a typed
 * `workerDraft` accumulator.
 *
 * `version` is bumped any time the persisted shape changes
 * incompatibly; the loader returns `null` on mismatch (forces a fresh
 * start). Proper migrations live in the gateway-backed draft store.
 */
export interface HireDraftState {
  /** Stored-schema version. Increment on incompatible shape changes. */
  version: 1;
  /** 1-based step the user has reached. 6 is the celebration / done. */
  step: 1 | 2 | 3 | 4 | 5 | 6;
  /** Step-1 free-text problem statement. */
  problem?: string;
  /** Step-2..5 answer bag — keyed by question id; opaque to the shell. */
  answers?: Record<string, unknown>;
  /** Accumulated worker draft (name, persona, role) populated by steps 3–5. */
  workerDraft?: WorkerDraft;
  /** Unix epoch ms of the most recent save. Used for the 7-day TTL. */
  ts: number;
}

/**
 * Voice profile inferred for the worker — drives reply tone in Step 4
 * preview. v1 ships a fixed default (warm/medium); the architectural
 * target reads operator's prior messages and infers a starting
 * profile (`mission-enterprise-genesis-voice-inference`).
 */
export interface WorkerVoiceProfile {
  tone: "warm" | "neutral" | "formal";
  formality: "low" | "medium" | "high";
  signaturePhrases: string[];
}

/** A single scheduled duty surfaced in Step 4 for editing. */
export interface WorkerDuty {
  label: string;
  schedule: { kind: "cron" | "event"; spec: string };
  defaultEnabled: boolean;
}

/** One inbound-context → suggested-reply pair for the Step 4 preview. */
export interface WorkerExampleReply {
  inboundContext: string;
  /** Generic field name — applies regardless of the worker's `suggestedName`. */
  mariahResponse: string;
}

/**
 * Server-side worker spec generated in Step 3 by Genesis-lite and
 * edited through Steps 4–5. Fields are populated all-at-once by the
 * draft-worker mutation; the gateway hire endpoint validates
 * required-ness on Step 6 submit.
 *
 * The v1 type is locked by the E2-d brief — Steps 4..i depend on this
 * exact shape. Any change here is a coordinated reshape across the
 * remaining E2 chain.
 */
export interface WorkerDraft {
  suggestedName: string;
  persona: {
    tagline: string;
    avatarKind: "initials" | "generated" | "uploaded";
    voiceProfile: WorkerVoiceProfile;
  };
  /** 2–3 sentences describing the worker's role and remit. */
  charter: string;
  duties: WorkerDuty[];
  exampleReplies: WorkerExampleReply[];
  approvalPolicy: {
    level: "conservative" | "balanced" | "aggressive";
  };
  suggestedVertical: string;
}

/** Tag the localStorage key in one place so renames don't drift. */
export const HIRE_DRAFT_STORAGE_KEY = "skycot.hireDraft";

/** 7-day TTL — matches the v1 brief; v1.5 swaps to server-enforced. */
export const HIRE_DRAFT_TTL_MS = 7 * 24 * 60 * 60 * 1000;

/** Current persisted-schema version. */
export const HIRE_DRAFT_VERSION = 1 as const;
