/**
 * Waitlist (early-access) signup API client.
 *
 * ⚠️ BACKEND FOLLOW-UP REQUIRED — the gateway route this posts to,
 * `POST /api/v1/waitlist`, DOES NOT EXIST YET. There is currently no
 * `routes_waitlist*.rs` handler in `crates/argo-cli/src/handlers/gateway/`.
 * Until that handler ships, every call here will resolve to a 404 (or a
 * network error in a static export) and `joinWaitlist` surfaces that as a
 * thrown `ApiError`. The landing-page form deliberately does NOT fake a
 * success state on failure — per the NO-THEATER rule (DEC-007), a draft
 * marketing surface must not pretend a submission landed when no backend
 * recorded it.
 *
 * Backend brief to author (separate PR):
 *   - `POST /api/v1/waitlist` { email, source?, referrer? } → 202 { queued: true }
 *   - Public/anonymous-safe: stricter rate-limit + bot/abuse guard than the
 *     authenticated surface (this route accepts unauthenticated traffic).
 *   - Persist to a `waitlist_signups` table (additive migration) or forward
 *     to the operator's email-capture provider.
 *   - Dedupe on email; idempotent re-submit returns 202.
 *
 * Wire-shape is intentionally minimal so the Rust handler can match it
 * without renaming fields when it lands.
 */

import { apiPost } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";

export interface WaitlistSignupRequest {
  email: string;
  /** Where the signup originated, e.g. "landing-hero" or "landing-final". */
  source?: string;
}

export interface WaitlistSignupResponse {
  queued: boolean;
}

/** RFC-5322-lite — good enough to reject obvious typos before a round-trip. */
const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

export function isValidEmail(value: string): boolean {
  return EMAIL_RE.test(value.trim());
}

/**
 * Submit an email to the early-access waitlist.
 *
 * Throws `ApiError` on any non-2xx (including the 404 that the
 * not-yet-built backend currently returns) so the form can show a real
 * error instead of a fabricated success. When the backend lands, a 202
 * `{ queued: true }` resolves normally.
 */
export async function joinWaitlist(
  req: WaitlistSignupRequest,
): Promise<WaitlistSignupResponse> {
  const email = req.email.trim();
  if (!isValidEmail(email)) {
    throw new ApiError(400, "Enter a valid email address.", "INVALID_EMAIL");
  }
  const raw = await apiPost<unknown>("/api/v1/waitlist", {
    email,
    source: req.source ?? "landing",
  });
  if (raw && typeof raw === "object" && typeof (raw as { queued?: unknown }).queued === "boolean") {
    return raw as WaitlistSignupResponse;
  }
  // Backend present but unexpected shape — treat as queued only if the
  // call itself succeeded (apiPost already threw on non-2xx). A 202 with
  // an empty body lands here.
  return { queued: true };
}
