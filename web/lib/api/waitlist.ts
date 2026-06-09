/**
 * Waitlist (early-access) signup API client.
 *
 * The gateway route this posts to, `POST /api/v1/waitlist`, is LIVE
 * (`crates/argo-cli/src/handlers/gateway/routes_waitlist.rs`, mounted in
 * `routes.rs` ahead of the bearer-auth gate). It is public/anonymous-safe:
 * the email is validated, run through the shared per-IP signup rate-limiter
 * and honeypot guard, then inserted into the `waitlist_signups` table
 * (additive migration v88) with `ON CONFLICT(email) DO NOTHING`. Submitting
 * an email returns an idempotent `202 { queued: true }`, so a duplicate
 * re-submit is a no-op that still reports success. A non-2xx therefore means
 * a genuine routing/deploy fault, not an expected "not built yet" state, and
 * `joinWaitlist` surfaces it as a thrown `ApiError` rather than faking a
 * success banner (NO-THEATER, DEC-007).
 *
 * Wire-shape is minimal and matches the Rust handler's fields directly.
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
 * Resolves `{ queued: true }` on the live route's `202` (both an explicit
 * `{ queued: true }` body and an empty `202` body). Throws `ApiError` on any
 * non-2xx so the form can show a real error instead of a fabricated success.
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
