/**
 * GTM Agent (Grant) API client.
 *
 * Calls `POST /api/v1/gtm/draft-followup` — Grant's first real
 * responsibility: given a waitlist signup, he drafts a short,
 * brand-safe inbound follow-up via the live LLM (the operator's Claude
 * subscription — DEC-012 `claude-code-cli`, Sonnet floor DEC-013),
 * re-scans the draft through the same truth-boundary the Pitch Agent
 * uses, and routes it to the approval queue (`/approvals`) — never
 * auto-sends.
 *
 * Wire-shape mirror of `crates/argo-cli/src/handlers/gateway/
 * routes_gtm_draft_followup.rs`. Any change to the Rust response struct
 * must land here in the same PR.
 */

import { apiPost } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";

/**
 * Truth-boundary verdict on Grant's draft.
 * - `Allow` — the draft is brand-safe and ready for the approval queue.
 * - `Escalate` — the draft touched a guarded topic (valuation, revenue,
 *   customer-count …); the runtime swapped in a "held for founder
 *   review" note instead of an unsafe outbound.
 * - `Block` — reserved (forward-compat); treated like Escalate in the UI.
 */
export type GtmDisposition = "Allow" | "Escalate" | "Block";

/** Where the draft came from. Always `llm` today (drafting is generative). */
export type GtmSource = "llm" | "canonical";

export interface GtmSignup {
  name: string;
  email?: string;
  context?: string;
}

/** Provider-reported token counts for the draft. */
export interface GtmTokens {
  input?: number | null;
  output?: number | null;
}

/** Grant's explainable lead-qualification tier. */
export type GtmTier = "Hot" | "Warm" | "Cold";

/**
 * Grant's lead triage — deterministic + explainable (the LLM writes the
 * draft; the qualification is transparent signal extraction). `signals`
 * are the human-readable reasons behind the tier.
 */
export interface GtmQualification {
  tier: GtmTier;
  rationale: string;
  signals: string[];
}

export interface GtmDraftResponse {
  /** Always "Grant". */
  agent: string;
  /** Always "inbound-growth". */
  responsibility: string;
  /** Grant's lead triage. Optional for forward-compat with older gateways. */
  qualification?: GtmQualification;
  /** The drafted follow-up (or the safe escalation note on Escalate). */
  draft: string;
  disposition: GtmDisposition;
  /** Forward-compat: gateways always set this to `llm` today. */
  source?: GtmSource;
  /** Model id the draft was generated with. */
  model?: string;
  /**
   * The approval-queue id the draft was filed under. Present when the
   * gateway wrote the approval (the normal path); null only if the
   * approval write degraded.
   */
  approval_id?: string | null;
  /** Workspace tenant the approval landed in. */
  tenant?: string | null;
  signup?: { name: string; email: string };
  tokens?: GtmTokens;
}

const DISPOSITIONS: ReadonlySet<GtmDisposition> = new Set([
  "Allow",
  "Escalate",
  "Block",
]);

function isGtmDraftResponse(raw: unknown): raw is GtmDraftResponse {
  if (!raw || typeof raw !== "object") return false;
  const v = raw as Record<string, unknown>;
  if (typeof v.draft !== "string") return false;
  if (typeof v.disposition !== "string") return false;
  if (!DISPOSITIONS.has(v.disposition as GtmDisposition)) return false;
  return true;
}

/**
 * Ask Grant to draft an inbound follow-up for one waitlist signup.
 * Returns the gateway's full response; the caller renders the draft +
 * disposition badge and the approval-queue link.
 *
 * Throws `ApiError` on any non-2xx, malformed payload, or network
 * failure so the console surface can show a single error banner.
 */
export async function draftFollowup(signup: GtmSignup): Promise<GtmDraftResponse> {
  const inner: Record<string, string> = {
    name: signup.name,
    email: signup.email ?? "",
  };
  if (signup.context && signup.context.trim().length > 0) {
    inner.context = signup.context.trim();
  }
  const raw = await apiPost<unknown>("/api/v1/gtm/draft-followup", { signup: inner });
  if (!isGtmDraftResponse(raw)) {
    throw new ApiError(
      502,
      "GTM endpoint returned an unexpected payload",
      "INVALID_GTM_RESPONSE",
    );
  }
  return raw;
}
