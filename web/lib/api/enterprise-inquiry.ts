/**
 * Enterprise Expression-of-Interest (EOI) capture API client.
 *
 * Posts a structured enterprise lead to `POST /api/v1/enterprise-inquiry`
 * (handler: `crates/argo-cli/src/handlers/gateway/routes_enterprise_inquiry.rs`,
 * WS6-2). This replaces the old `mailto:` "Talk to sales" CTA on `/pricing`
 * and `/contact` so inbound enterprise interest is persisted + auditable —
 * the front door for the EOI→MOU→contract motion.
 *
 * Per NO-THEATER (DEC-007) the form does NOT fake success: a real 2xx resolves
 * normally; any non-2xx throws `ApiError` so the form shows a real error
 * instead of a fabricated "we got it".
 *
 * Wire-shape mirrors the Rust handler's `EnterpriseInquiryBody`:
 *   { company, name, email, seats?, message? } → 202 { queued: true }
 */

import { apiPost } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";

export interface EnterpriseInquiryRequest {
  company: string;
  name: string;
  email: string;
  /** Optional free-text seat estimate, e.g. "250" or "500-1000". */
  seats?: string;
  /** Optional context for the sales conversation. */
  message?: string;
}

export interface EnterpriseInquiryResponse {
  queued: boolean;
}

/** RFC-5322-lite — good enough to reject obvious typos before a round-trip. */
const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

export function isValidEmail(value: string): boolean {
  return EMAIL_RE.test(value.trim());
}

/**
 * Submit an enterprise inquiry.
 *
 * Validates the three required fields client-side (so an obviously-incomplete
 * form never hits the backend), then POSTs. Throws `ApiError` on any non-2xx
 * so the form surfaces a real failure. A 202 `{ queued: true }` — or a 2xx
 * with an empty body — resolves as queued.
 */
export async function submitEnterpriseInquiry(
  req: EnterpriseInquiryRequest,
): Promise<EnterpriseInquiryResponse> {
  const company = req.company.trim();
  const name = req.name.trim();
  const email = req.email.trim();
  if (!company) {
    throw new ApiError(400, "Enter your company name.", "MISSING_COMPANY");
  }
  if (!name) {
    throw new ApiError(400, "Enter your name.", "MISSING_NAME");
  }
  if (!isValidEmail(email)) {
    throw new ApiError(400, "Enter a valid work email address.", "INVALID_EMAIL");
  }
  const seats = req.seats?.trim();
  const message = req.message?.trim();
  const raw = await apiPost<unknown>("/api/v1/enterprise-inquiry", {
    company,
    name,
    email,
    ...(seats ? { seats } : {}),
    ...(message ? { message } : {}),
  });
  if (
    raw &&
    typeof raw === "object" &&
    typeof (raw as { queued?: unknown }).queued === "boolean"
  ) {
    return raw as EnterpriseInquiryResponse;
  }
  // Backend present but unexpected shape — treat as queued only because the
  // call itself succeeded (apiPost already threw on non-2xx). A 202 with an
  // empty body lands here.
  return { queued: true };
}
