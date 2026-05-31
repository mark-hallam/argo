/**
 * Marketing teammate (Marcus) API client.
 *
 * Calls `POST /api/v1/marketing/draft-response` — Marcus's standing
 * responsibility (DEC-028, the third hero teammate after Portia + Grant):
 * the operator pastes a mention / URL (reply mode) or gives a topic
 * (article mode), picks a channel (X short / LinkedIn long-form), and
 * Marcus drafts a truth-bounded piece in Mark's documented voice via the
 * live LLM (the operator's Claude subscription — DEC-012 `claude-code-cli`,
 * Sonnet floor DEC-013). The draft is re-scanned through the same
 * truth-boundary the Pitch Agent uses and filed to the approval queue
 * (`/approvals`).
 *
 * DRAFT-ONLY: Marcus NEVER posts. There is no posting integration in v0
 * (greenfield per the marketing-teammate plan) — the operator hand-posts
 * the approved draft.
 *
 * Wire-shape mirror of `crates/argo-cli/src/handlers/gateway/
 * routes_marketing_draft_response.rs`. Any change to the Rust response
 * struct must land here in the same PR.
 */

import { apiPost } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";

/**
 * Truth-boundary verdict on Marcus's draft.
 * - `Allow` — the draft is brand-safe and ready for the approval queue.
 * - `Escalate` — the draft touched a guarded topic (valuation, revenue,
 *   customer-count …); the runtime swapped in a "held for founder
 *   review" note instead of an unverified public claim.
 * - `Block` — reserved (forward-compat); treated like Escalate in the UI.
 */
export type MarketingDisposition = "Allow" | "Escalate" | "Block";

/** What shape of copy Marcus produces. */
export type MarketingMode = "reply" | "article";

/** Which platform's format the draft targets (NOT a posting target). */
export type MarketingChannel = "x" | "linkedin";

/** Where the draft came from. Always `llm` today (drafting is generative). */
export type MarketingSource = "llm";

/** Provider-reported token counts for the draft. */
export interface MarketingTokens {
  input?: number | null;
  output?: number | null;
}

export interface MarketingDraftInput {
  mode: MarketingMode;
  channel: MarketingChannel;
  /** Reply mode: the pasted mention / post / URL. Required in reply mode. */
  source?: string;
  /** Article mode: the topic. Required in article mode. */
  topic?: string;
}

export interface MarketingDraftResponse {
  /** Always "Marcus". */
  agent: string;
  /** Always "marketing". */
  responsibility: string;
  mode: MarketingMode;
  channel: MarketingChannel;
  /** The drafted copy (or the safe escalation note on Escalate). */
  draft: string;
  disposition: MarketingDisposition;
  /** Forward-compat: gateways always set this to `llm` today. */
  source?: MarketingSource;
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
  tokens?: MarketingTokens;
}

const DISPOSITIONS: ReadonlySet<MarketingDisposition> = new Set([
  "Allow",
  "Escalate",
  "Block",
]);

function isMarketingDraftResponse(raw: unknown): raw is MarketingDraftResponse {
  if (!raw || typeof raw !== "object") return false;
  const v = raw as Record<string, unknown>;
  if (typeof v.draft !== "string") return false;
  if (typeof v.disposition !== "string") return false;
  if (!DISPOSITIONS.has(v.disposition as MarketingDisposition)) return false;
  return true;
}

/**
 * Ask Marcus to draft a reply or article for X / LinkedIn. Returns the
 * gateway's full response; the caller renders the draft + disposition
 * badge and the approval-queue link. The draft is NEVER posted.
 *
 * Throws `ApiError` on any non-2xx, malformed payload, or network
 * failure so the console surface can show a single error banner.
 */
export async function draftResponse(
  input: MarketingDraftInput,
): Promise<MarketingDraftResponse> {
  const inner: Record<string, string> = {
    mode: input.mode,
    channel: input.channel,
  };
  if (input.source && input.source.trim().length > 0) {
    inner.source = input.source.trim();
  }
  if (input.topic && input.topic.trim().length > 0) {
    inner.topic = input.topic.trim();
  }
  const raw = await apiPost<unknown>("/api/v1/marketing/draft-response", {
    request: inner,
  });
  if (!isMarketingDraftResponse(raw)) {
    throw new ApiError(
      502,
      "Marketing endpoint returned an unexpected payload",
      "INVALID_MARKETING_RESPONSE",
    );
  }
  return raw;
}
